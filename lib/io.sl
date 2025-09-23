(use '(utils len))

;
; Flags.
; 

(val READ_ONLY  x0000)
(val WRITE_ONLY x0001)
(val READ_WRITE x0002)

;
; Modes.
; 

(val NON_BLOCKING x0004)
(val APPEND       x0008)
(val SYNC         x0080)
(val CREATE       x0200)
(val TRUNCATE     x0400)
(val EXCLUSIVE    x0800)

;
; Common streams.
;

(val STDIN  0)
(val STDOUT 1)
(val STDERR 2)

;
; Open and close.
; 

(ext open nil ((path  . string)
               (flags . integer)
               (mode  . integer))
  "Open or create a file at PATH with FLAGS and MODE."
  integer)

(ext close nil ((fd . integer))
  "Close the file descriptor FD."
  void)

;
; Read and write.
; 

(ext read nil ((fd  . integer)
               (buf . bytes)
               (len . integer))
  "Attempt to read LEN from FD into BUF."
  integer)

(ext write nil ((fd  . integer)
                (buf . bytes)
                (len . integer))
  "Attempt to write LEN from FD into BUF."
  integer)

;
; Print.
;

(def print (value)
  "Write string VALUE to STDOUT."
  (let ((chars  . (split value))
        (length . (len chars))
        (bytes  . (bytes chars)))
    (write STDOUT bytes length)))

(def println (value)
  "Write string VALUE and a newline to STDOUT."
  (print value)
  (print "\n"))
