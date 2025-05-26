(def map (f v)
  (if v (cons (f (car v)) (map f (cdr v)))))

(def main ()
  (map (\ (a) (+ a 1)) '(1 2 3)))
  
