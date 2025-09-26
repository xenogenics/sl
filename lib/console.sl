(use
  '(io STDOUT write)
  '(iterators foldr iter)
  '(lang |> match)
  '(list len))

;
; String escape.
;

(val STYLE_TABLE (
   ; Style
  (normal     . "0")
  (bold       . "1")
  (italic     . "3")
  (underlined . "4")
  (reversed   . "7")
  (striked    . "9")
   ; Colors
  (black      . "0;30")
  (red        . "0;31")
  (green      . "0;32")
  (orange     . "0;33")
  (blue       . "0;34")
  (purple     . "0;35")
  (cyan       . "0;36")
  (gray+      . "0;37")
  (gray-      . "1;30")
  (red+       . "1;31")
  (green+     . "1;32")
  (yellow     . "1;33")
  (blue+      . "1;34")
  (purple+    . "1;35")
  (cyan+      . "1;36")
  (white      . "1;37")
))

(def esc (code value)
  "Escape the string VALUE with CODE."
  (|>
    (unpack "[0m")
    (cons (chr 27))
    (conc (unpack value))
    (cons ^m)
    (conc (unpack code))
    (cons ^[)
    (cons (chr 27))
    str))

(def style (style value)
  (match style
    ~(foldr
       (\ (e acc)
         (let ((iden . (car e))
               (code . (cdr e)))
           (cons `(,iden . (esc ,code value)) acc) 
       ))
       STYLE_TABLE
       nil
     )))

;
; Print.
;

(def prin1 (value)
  "Write string VALUE to STDOUT."
  (let ((chars  . (unpack value))
        (length . (len chars))
        (bytes  . (bytes chars)))
    (write STDOUT bytes length)))

(def print @
  "Write all arguments to STDOUT."
  (iter prin1 @))

(def prin1ln (value)
  "Write string VALUE and a newline to STDOUT."
  (prin1 value)
  (prin1 "\n"))

(def println @
  "Write string VALUE and a newline to STDOUT."
  (iter prin1 @)
  (prin1 "\n"))
