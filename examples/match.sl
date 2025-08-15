(use '(lang match foldl rev))

(def main ()
  (match '(1 2)
    ((1 3) . T)
    ((1 2) . (+ 2 1))
    (_ . 0)))
