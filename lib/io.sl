(use '(iterators iter))

(def prinl LST
  "Print the string version of each element of LST."
  (let ((write . (\ (e) (syscall WRITE 1 e))))
    (iter (\ (e) (write (str e))) LST)
    (write "\n")))
