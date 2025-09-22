(use '(iterators rev) '(lang |> cond unless))

(def >chars (expr)
  "Convert EXPR into its list of characters representation."
  (cond expr
    (chr? . (cons it nil))
    (lst? . (let ((rec . (\ (e)
                           (if e
                             (let ((hd . (>chars (car e)))
                                  (tl . (cdr e)))
                               (if tl
                                 (conc hd (cons ^\s (self tl)))
                                 (conc hd (self tl))))
                              (cons ^) nil)))
                         ))
              (cons ^( (rec it))))
    (num? . (if (= it 0)
              "0"
              (let ((digits . (\ (n)
                                (unless (= n 0)
                                  (cons
                                    (chr (+ 48 (% n 10)))
                                    (self (/ n 10))
                                  ))
                              )))
                (|> it digits rev)
              )))
    (str? . (split it))
    (sym? . (split it))
    (tru? . (cons ^T nil))
))

(def >str (expr)
  "Convert EXPR into a string."
  (str (>chars expr)))
