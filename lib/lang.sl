(load '(iterators foldl))

(def |> (val . lst)
  "Evaluate the first element of LST with VAL and pass down the result."
  (foldl (\ (acc elt) (elt acc)) val lst))
