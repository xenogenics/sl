(use '(lang |>))

(def main ()
  "Validate the fluent operator."
  (|> 1 (+ 1) (+ 2)))
