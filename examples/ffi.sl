(use 'io '(iterators take)) 

(def main ()
  (let ((fd  . (open "/tmp/hello" READ_ONLY 0))
        (buf . (bytes 8))
        (res . (read fd buf 8)))
    (close fd)
    `(,res ,(str (take (- res 1) (unpack buf))))
  ))
          
