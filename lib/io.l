(load '(iterators iter))

(def prinl LST
  "Print the string version of each element of LST."
  (let ((write . (\ (e) (syscall WRITE 1 e)))
        (to_string . (\ (e)
          (if (lst? e)
            (write e)
            (write "X")))))
    (iter (\ (e) (to_string e)) LST)
    (write "\n")))
