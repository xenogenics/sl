(use '(lang cond foldl rev))

(def main ()
  (cond '(1 2)
    ((num? it) . T)
    ((lst? it) . (+ 2 1))
    (T . 0)))
