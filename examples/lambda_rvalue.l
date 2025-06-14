(def add (a)
  (\ (b) (+ a b)))

(def main ()
  ((add 1) 1))
