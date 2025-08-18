(def I (x)
  "Identity (I) combinator"
  x)

(def K (x)
  "Constant (K) combinator"
  (\ (y) x))

(def S (x)
  "Substitution (S) combinator"
  (\ (y) (\ (z) ((x z) (y z)))))

(def B (x)
  "Composition (B) combinator"
  (\ (y) (\ (z) (x (y z)))))

(def C (x)
  "Flip (C) combinator"
  (\ (y) (\ (z) ((x z) y))))
  
(def W (x)
  "Duplication (W) combinator"
  (\ (y) ((x y) y)))

(def Y (f)
  "Fixed-point (Y) combinator"
  ((\ (x) (f (\ (y) ((x x) y))))
   (\ (x) (f (\ (y) ((x x) y))))))
