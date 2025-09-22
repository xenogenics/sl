(use '(lang cond foldl rev))

(def main ()
  (cond '(1 2)
    (num? . T)
    (lst? . (+ 2 1))
    (_    . 0)
  ))
