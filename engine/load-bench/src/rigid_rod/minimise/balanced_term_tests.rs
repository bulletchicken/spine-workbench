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
