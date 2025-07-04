(def check_symbol (val)
  "Check VAL using MATCH."
  (match (car val)
    (CMD_A . 1)
    (CMD_B . 2)
    (CMD_C . 3)
    (CMD_D . 4)
    (_ . -1)))

(def main ()
  (check_symbol '(CMD_A)))
