(use '(lang foldl))

(def len (lst)
  "Return the length of the list LST."
  (if (lst? lst)
    (foldl (\ (acc e) (+ acc 1)) 0 lst)))
