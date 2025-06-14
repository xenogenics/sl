(def add (a b) (+ a b))

(def main ()
	(let ((incr_by_one . (add 1))
	      (incr_by_two . ((\ (a b) (+ a b)) 2)))
		 (incr_by_two (incr_by_one 2))))
