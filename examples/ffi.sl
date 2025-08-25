(ext open nil ((path  . string)
               (flags . integer)
               (mode  . integer))
  "Open or create a file for reading or writing"
  integer)

(def main ()
  (open "/tmp/hello" 0 0))
        
