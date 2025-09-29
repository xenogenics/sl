(use
  '(iterators foldl foldr)
  '(lang match))

;
; Item accessors.
; 

(mac caar (x)
  "Call CAR CAR on X."
  `(car (car ,x)))

(mac cddr (x)
  "Call CDR CDR on X."
  `(cdr (cdr ,x)))

(mac cadar (x)
  "Call CAR CDR CAR on X."
  `(car (cdr (car ,x))))

(mac caddr (x)
  "Call CDR CDR CAR on X."
  `(car (cdr (cdr ,x))))

(mac cadddr (x)
  "Call CDR CDR CDR CAR on X."
  `(car (cdr (cdr (cdr ,x)))))

(mac cadr (x)
  "Call CDR CAR on X."
  `(car (cdr ,x)))

(mac cdar (x)
  "Call CAR CDR on X."
  `(cdr (car ,x)))

;
; Associative lists.
; 

(def get (k lst)
  "Return the value for K in the association list LST."
  (if (not (nil? lst))
    (if (= (caar lst) k)
      (cdar lst)
      (self k (cdr lst)))))

(def remove (k lst)
  "Remove the entry for K in the association list LST."
  (foldr
    (\ (e acc)
      (let ((_k . (car e))
            (_v . (cdr e)))
        (if (= k _k) acc (cons (cons _k _v) acc))))
    lst nil))

(def replace (k v lst)
  "Replace the entry for K with V in the association list LST."
  (foldr
    (\ ((_k . _v) acc)
      (if (= k _k)
        (cons (cons k v) acc)
        (cons (cons _k _v) acc)
    ))
    lst nil))

;
; Misc utilities.
; 

(def dedup (cmp lst)
  "Deduplicate list LST using comparator CMP."
  ;
  ; The comparator is expected to return a pair (CMD . VAL) where CMD is:
  ; 
  ; * APPEND : append VAL to the accumulator
  ; * REPLACE: replace the head of the accumulator by VAL
  ; * SKIP   : skip VAL
  ;
  (foldr (\ (elt acc)
   (if (nil? acc)
     (cons elt acc)
     (let (((cmd . value) . (cmp elt (car acc))))
       (match cmd
         (APPEND  . (cons value acc))
         (REPLACE . (cons value (cdr acc)))
         (SKIP    . acc)))))
  lst nil))

(def flatten (lst)
  "Flatten k-deep list LST into a 1-deep list."
  (if (lst? lst)
    (foldr (\ (lst acc) (conc (flatten lst) acc)) lst nil)
    (cons lst nil)
  ))

(def has (x lst)
  "Check if X is in LST."
  (foldl (\ (acc e) (or (= e x) acc)) nil lst))

  
(def insert (cmp a lst)
  "Insert A into ordered list LST using comparator CMP."
  (if lst
    (let ((cur . (car lst))
          (nxt . (cdr lst)))
      (if (cmp a cur)
        (cons a (cons cur nxt))
        (cons cur (insert cmp a nxt))))
    (cons a nil)
  ))

(def last (lst)
  "Return the last item of LST."
  (if (nil? (cadr lst))
    (car lst)
    (last (cdr lst))))

(def len (lst)
  "Return the length of the list LST."
  (if (lst? lst)
    (foldl (\ (acc e) (+ acc 1)) 0 lst)))

(def split (fun lst)
  "Split the list LST according to the predicate FUN."
  (let (((stack . result) . (foldr
                              (\ (e (stack . result))
                                    (if (fun e)
                                       (cons nil (cons (cons e stack) result))
                                       (cons (cons e stack) result)))
                              lst
                              nil
                            )))
    (cons stack result)
  ))
  
(def sort (cmp lst)
  "Insertion-sort of list LST using comparator CMP."
  (foldl (\ (acc e) (insert cmp e acc)) nil lst))
