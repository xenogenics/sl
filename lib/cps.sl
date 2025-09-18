(use 'lang)

;
; Continuation-passing style translator.
; 
; Inspired by: https://okmij.org/ftp/Scheme/callcc-calc-page.html
;
; Returns a closure of the form: (\ (k) ...) where k is the next continuation.
; To produce the result, the closure can be applied to the (id) function.
; 

(mac cps (expr)
  "Translate the EXPR expression into continuation-passing style." 
  (match expr
    ;
    ; Lambda.
    ;
    ; Apply the continuation k to the lambda. The content of the lambda is
    ; rewritten in continuation-passing style.
    ; 
    ((\ (_) _) . (let ((arg . (car (cdr it)))
                       (exp . (car (cdr (cdr it)))))
                   `(\ (k) (
                      k
                      (\ ,arg ,(cps exp))
                    ))
                 ))
    ;
    ; Single argument application.
    ;
    ; Translate each element of the application to CPS sequentially, then
    ; apply them in order. The continuation k is passed as the last argument,
    ; so it is assumed that the applicator is compatible with CPS.
    ; 
    ((_ _) . (let ((e0 . (car it))
                   (e1 . (car (cdr it))))
               `(\ (k) (
                   ,(cps e0)
                   (\ (e0) (
                      ,(cps e1)
                       (\ (e1)
                          ((e0 e1) k))
                ))))
             ))
    ;
    ; Two arguments application.
    ;
    ; Translate each element of the application to CPS sequentially, then
    ; apply them in order. The continuation k is passed as the last argument,
    ; so it is assumed that the applicator is compatible with CPS.
    ; 
    ((_ _ _) . (let ((e0 . (car it))
                     (e1 . (car (cdr it)))
                     (e2 . (car (cdr (cdr it)))))
                 `(\ (k) (
                     ,(cps e0)
                     (\ (e0) (
                        ,(cps e1)
                        (\ (e1) (
                           ,(cps e2)
                           (\ (e2)
                              ((e0 e1 e2) k))
                  ))))))
               ))
    ;
    ; (call/cc p).
    ;
    ; k0: continuation with the doublet applicative form for (call/cc p)
    ; pp: the applicative argument of call/cc 
    ; k1: argument to pass the final continuation
    ; kn: the next continuation
    ;
    ; The form (\ (a) (\ (k1) (k1 a))) is the "current continuation" passed to
    ; p. p will produce a result, turning it into (\ (k1) (k1 X)), which applied
    ; to k will produce (k X), with k being the outer continuation. 
    ;
    (call/cc . `(\ (k0)
                   (k0 (\ (pp)
                          (\ (kn) (
                             (pp (\ (a) (\ (k1) (k1 a))))
                             kn
                          ))))))
    ;
    ; Any other value.
    ; 
    (_ . `(\ (k) (k ,it)))
  ))

;
; Closure capture.
;
; Meant to be used in conjunction with (catch/cc), it will cause (cps) to
; return the current continuation in a CPS closure: (\ (k) ... cc ...).
;
; To make use of the captured continuation, it must be first unpacked using
; for instance the (id) function: ((cps (... (catch/cc capture))) id).
;
; It then can be applied to any value expected by the continuation:
;
; (let ((cc . ((cps (... (catch/cc capture)) id))))
;     (cc some-value))
; 

(def capture (k0)
   "Capture the current continuation K0 by returning it."
   (\ (k1)           ; k1 is the next continuation, we need to capture it
      (\ (v)         ; v is the value we want to fill in later
         ((k0 v) k1) ; apply the current continuation to v and pass it along
      )))

;
; Identity function.
; 

(def id (x)
   "The identity function that return its argument."
   x)
