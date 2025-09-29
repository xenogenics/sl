(use
  '(iterators foldl)
  '(lang cond match)
  'list)

 (def args:arity (cnt lst)
  "Compute the arity of a give argument list LST."
  (cond lst
    (nil? . (cons 'exact cnt))
    (sym? . (cons 'at-least cnt))
    (lst? . (self (+ cnt 1) (cdr lst)))
  ))

;
; Command line arguments parsing.
;
; (args:parse argv
;   ("-h" . (\ () (usage)))
;   ("-f" . (\ (file) (do_something_with_file file))))
;
; The number of arguments for each flag is determined by the arity of the
; lambda expression in the schema. If the number of arguments does not match,
; an error is returned instead of processing the lambda expression.
; 

(mac args:parse (argv . schema)
  "Parse ARGV according to SCHEMA."
  (let ((argn . (foldr
                  (\ ((key . fun) acc)
                    (cons (cons key (args:arity 0 (cadr fun))) acc))
                    schema
                    nil
                )))
   `(let ((indx . (split (\ (e) (= (car (unpack e)) ^-)) (cdr ,argv)))
          (prsr . (\ (acc (key . values))
                    (let ((fun         . (match key ,@schema))
                          ((cmp . exp) . (get key (quote . ,argn)))
                          (vln         . (len values)))
                      (if (not fun)
                        (cons (cons key 'invalid-arg) acc)
                        (if (> exp vln)
                          (cons (cons key 'missing-args) acc)
                          (if (and (< exp vln) (= cmp 'exact))
                            (cons (cons key 'too-many-args) acc)
                            (cons (cons key (apply fun values)) acc)
                      )))))))
      (foldl prsr nil (rev indx)))
  ))
