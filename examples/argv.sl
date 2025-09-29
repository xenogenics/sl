(use 'args 'console)

(def usage ()
  "Example of CLI usage."
  (println "Usage:")
  (println "    -H         Print this message")
  (println "    -E value   Some one-value flag")
  (println "    -N value   Some two-value flag")
  'ok)

(def main(argv)
  "Example of CLI argument processing."
  (let ((result . (args:parse argv
                    ("-H" . (\ () (usage)))
                    ("-E" . (\ (v) v))
                    ("-N" . (\ (a b) (cons a b)))
                  )))
    result
  ))
