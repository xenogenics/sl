(use '(iterators take)) 

(ext open nil ((path  . string)
               (flags . integer)
               (mode  . integer))
  "Open or create a file at PATH with FLAGS and MODE."
  integer)

(ext read nil ((fd  . integer)
               (buf . bytes)
               (len . integer))
  "Attempt to read LEN from FD into BUF."
  integer)

(ext close nil ((fd . integer))
  "Close the file descriptor FD."
  void)

(def main ()
  (let ((fd  . (open "/tmp/hello" 0 0))
        (buf . (bytes 8))
        (res . (read fd buf 8)))
    (close fd)
    `(,res ,(str (take (- res 1) (unpack buf))))
  ))
           
