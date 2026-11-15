use super::*;

struct Quadratic(f64);

impl Model for Quadratic {
    type State = f64;

    fn dimension(&self) -> usize {
        1
    }

    fn evaluate(&self, x: &Self::State) -> Option<Evaluation> {
        Some(Evaluation {
            energy: 0.5 * self.0 * x * x,
            gradient: DVector::from_element(1, self.0 * x),
            hessian: DMatrix::from_element(1, 1, self.0),
            constraint: DVector::zeros(0),
            jacobian: DMatrix::zeros(0, 1),
        })
    }

    fn step(&self, x: &Self::State, increment: &DVector<f64>) -> Option<Self::State> {
        Some(x + increment[0])
    }
}

struct BalancedQuadratic(Quadratic);

impl Model for BalancedQuadratic {
    type State = f64;

    fn dimension(&self) -> usize {
        self.0.dimension()
    }

    fn evaluate(&self, state: &Self::State) -> Option<Evaluation> {
        self.0.evaluate(state)
    }

    fn step(&self, state: &Self::State, increment: &DVector<f64>) -> Option<Self::State> {
        self.0.step(state, increment)
    }

    fn balanced_term_scale(&self, _: &Self::State) -> Option<f64> {
        Some(10.0)
    }
}

#[test]
fn a_balance_scale_changes_only_the_relative_stationarity_bound() {
    let options = Options::default();
    let unchanged = minimise(&Quadratic(1.0), 1e-10, options);
    assert!(unchanged.settled);
    assert_eq!(unchanged.iterations, 1);
    assert_eq!(unchanged.state, 0.0);

    // At scale 10 the unchanged bound is 1e-12+2e-11*10=2.01e-10.
    // Only the point below it can be accepted without a Newton step.
    let balanced = BalancedQuadratic(Quadratic(1.0));
    let inside = minimise(&balanced, 1e-10, options);
    assert!(inside.settled);
    assert_eq!(inside.iterations, 0);
    assert_eq!(inside.state, 1e-10);
    let outside = minimise(&balanced, 3e-10, options);
    assert!(outside.settled);
    assert_eq!(outside.iterations, 1);
    assert_eq!(outside.state, 0.0);

    // Passing the scaled residual check still needs positive curvature.
    let maximum = minimise(&BalancedQuadratic(Quadratic(-1.0)), 1e-10, options);
    assert!(!maximum.settled);
    assert_eq!(maximum.reason, Reason::Singular);
}

#[test]
fn hook_and_horizontal_rows_preserve_the_individual_term_stopping_scale() {
    struct Rows {
        hook: bool,
        horizontal: bool,
    }
    impl Model for Rows {
        type State = DVector<f64>;
        fn dimension(&self) -> usize {
            3
        }
        fn evaluate(&self, x: &Self::State) -> Option<Evaluation> {
            let mut gradient = x.clone();
            let count = usize::from(self.hook) + usize::from(self.horizontal);
            let mut constraint = DVector::zeros(count);
            let mut jacobian = DMatrix::zeros(count, 3);
            let mut row = 0;
            for (axis, active, force) in [(0, self.hook, 5.0), (1, self.horizontal, 7.0)] {
                if active {
                    gradient[axis] += force;
                    constraint[row] = x[axis];
                    jacobian[(row, axis)] = 1.0;
                    row += 1;
                }
            }
            Some(Evaluation {
                energy: 0.5 * x.dot(x)
                    + if self.hook { 5.0 * x[0] } else { 0.0 }
                    + if self.horizontal { 7.0 * x[1] } else { 0.0 },
                gradient,
                hessian: DMatrix::identity(3, 3),
                constraint,
                jacobian,
            })
        }
        fn step(&self, x: &Self::State, increment: &DVector<f64>) -> Option<Self::State> {
            Some(x + increment)
        }
        fn balanced_term_scale(&self, _: &Self::State) -> Option<f64> {
            Some(10.0)
        }
    }
    // Each constrained force balances its multiplier exactly; the only KKT
    // residual is z. Regardless of row presence, the bound is 2.01e-10.
    for hook in [false, true] {
        for horizontal in [false, true] {
            let model = Rows { hook, horizontal };
            for (residual, iterations) in [(1e-10, 0), (3e-10, 1)] {
                let answer = minimise(
                    &model,
                    DVector::from_vec(vec![0.0, 0.0, residual]),
                    Options::default(),
                );
                assert!(answer.settled);
                assert_eq!(answer.iterations, iterations);
            }
        }
    }
}
