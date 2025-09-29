(use '(iterators foldl rev))

(mac |> (value . rem)
  "Evaluate the first element of LST with VAL and pass down the result."
  (foldl
    (\ (acc elt) (cons elt (cons acc nil)))
    value rem))

(mac cond (value . cases)
  "Evaluate VALUE and check IT against each CASE"
  (let ((rvrsd . (rev cases))
        (newif . (\ (acc e)
                   (let ((prd . (car e))
                         (rem . (cdr e)))
                     (if (wld? prd)
                       `(if T ,rem ,acc)
                       `(if (,prd it) ,rem ,acc))
                 )))
        (nestd . (foldl newif nil rvrsd)))
    `(let ((it . ,value)) ,nestd)
  ))

(mac match (value . cases)
  "Evaluate VALUE and check for structural equality against each CASE"
  (let ((rvrsd . (rev cases))
        (newif . (\ (acc e) `(if (= (quote ,@(car e)) it) ,(cdr e) ,acc)))
        (nestd . (foldl newif nil rvrsd)))
    `(let ((it . ,value)) ,nestd)
  ))

(mac unless (cond stmt)
  "Execute STMT if COND is NIL"
  `(if (not ,cond) ,stmt))
