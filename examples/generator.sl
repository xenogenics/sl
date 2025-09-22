(use 'cps)

;
; Generator builder.
;
; Build a generator function using call/cc.
; 

(def make-generator (proc)
  "Build a value generator using the generative procedure PROC."
  (let ((cps:cons . (\ (a b k) (k (cons a b)))))
    (cps (call/cc (\ (return)
      (proc
        (\ (x cont) (call/cc (\ (k)
          (return (cps:cons x (\ () (cont k)))))))
        return
      ))))))

;
; Range generator.
; 

(def range-gen (start end)
  "A range generator which generates a sequence between START and END."
  (make-generator
    (\ (yield return)
      (let ((loop . (\ (i) (if (< i end)
                      (let ((loop . (self)))
                        (yield i (\ (k) (k (loop (+ i 1))))))
                      (return T)
                    ))))
        (loop start)
      ))
  ))

;
; Continuation invocation.
; 
; The generator will return a result of the form (value . next), with value
; the current value generated and next the generator for the next value. This
; generator should be used in conjunction with the continuation invocation.
; 

(def invoke (k)
  "Invoke the continuation K produced by a generator."
  (((k) id) id))
