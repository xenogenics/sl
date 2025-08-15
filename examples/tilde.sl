(use '(iterators foldl))

(def test (value)
  (cond value
    ~(foldl
      (\ (acc e) (cons e acc))
      nil
      '((num? . 0) (lst? . 1))
      )
    (chr? . 2)
    ))

(def main ()
  (test ^\\))
