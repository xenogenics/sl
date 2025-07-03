(def count_args_r (A)
  (if A
     (+ 1 (count_args_r (cdr A)))
     0))

(def count_args A
  (count_args_r A))

(def main ()
    (count_args 1 2 3 4))
