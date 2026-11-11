# Made-up spines

`scorekeeper::datasets::made_up::score` generates one case from a recipe
and seed, then sends each of its four encoded films to a supplied finder.
The finder receives only pixels, dimensions, kind and geometry. It never
receives the answer key. No films need to be written to disk.

Corners and level names are scored against the generator's answer key.
Invisible corners are excluded from corner errors; their vertebrae stay
in found and naming counts. Error is reported in pixels, relative to
true body height, and in mm where the key supplies a scale. Non-square
pixels use both stored spacings when converting displacement to mm.

The key's anatomical curve angles differ from the AASCE benchmark's
three-column angles. Those benchmark fields are absent for this dataset;
the scorer does not pretend that the two angle definitions coincide.
Accuracy on generated films is not clinical accuracy.

VertebraeKeypoints50 and hand-marked case readers remain documented
skeletons. Their formats and reference-mark policies need verification
before scoring can use them.
