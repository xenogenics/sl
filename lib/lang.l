(load '(iterators foldl))

(def |> (VAL . LST)
  "Evaluate the first element of LST with VAL and pass down the result."
  (foldl (\ (ACC ELT) (ELT ACC)) VAL LST))
