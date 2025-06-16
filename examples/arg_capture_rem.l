(def select (A B . C)
  (if C A B))

(def main ()
    (let ((f . (select 1))
          (a . (f 2))
          (b . (f 2 3)))
      (+ a b)))
