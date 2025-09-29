(use
  '(iterators foldl foldr repeat rev take)
  '(lang |> cond unless)
  '(list len split))

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
          acc0 (unpack e0)
      ))
      (unpack x) @
  )))

(def $+ (x y)
  "Concatenate two strings."
  (append x y))

;
; Splitting.
;

(def $/ (c value)
  "Split string VALUE at character C."
  (|>
    value
    unpack
    (split (\ (e) (= e c)))
    (foldl (\ (acc e) (cons (str e) acc)) nil)
    rev
  ))

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
    (cmp (unpack stra) (unpack strb))
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
    (str? . (unpack it))
    (sym? . (unpack it))
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
  (let ((chars . (unpack strn))
        (delta . (- wlen (len chars)))) 
    (if (>= delta 0)
      ($+ strn (str (repeat delta ^\s)))
      (str (take wlen chars)))))
      
(def +> (wlen strn)
  "Right-pad STRN by WLEN spaces."
  (let ((chars . (unpack strn))
        (delta . (- wlen (len chars)))) 
    (if (>= delta 0)
      ($+ (str (repeat delta ^\s)) strn)
      (str (rev (take wlen (rev chars)))))))
