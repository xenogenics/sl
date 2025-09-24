(use
  '(iterators foldl foldr repeat rev take)
  '(lang |> cond unless)
  '(list len))

;
; Concatenation.
;

(def append (x . @)
  "Concatenate multiple string."
  (str
    (foldl
      (\ (acc0 e0)
        (foldr
          (\ (e1 acc1) (cons e1 acc1))
          acc0 (split e0)
      ))
      (split x) @
  )))

(def $+ (x y)
  "Concatenate two strings."
  (append x y))

;
; Comparison.
;

(def $< (stra strb)
  "Strict less operator."
  (let ((cmp . (\(ba bb)
                 (and (not (nil? bb)) (or
                   (nil? ba)
                   (let ((a . (car ba))
                         (b . (car bb)))
                     (or (< a b) (and
                       (= a b)
                       (self (cdr ba) (cdr bb))))
                    ))))))
    (cmp (split stra) (split strb))
  ))

;
; Formatting.
; 

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

;
; Padding.
;

(def <+ (wlen strn)
  "Left-pad STRN by WLEN spaces."
  (let ((chars . (split strn))
        (delta . (- wlen (len chars)))) 
    (if (>= delta 0)
      ($+ strn (str (repeat delta ^\s)))
      (str (take wlen chars)))))
      
(def +> (wlen strn)
  "Right-pad STRN by WLEN spaces."
  (let ((chars . (split strn))
        (delta . (- wlen (len chars)))) 
    (if (>= delta 0)
      ($+ (str (repeat delta ^\s)) strn)
      (str (rev (take wlen (rev chars)))))))
