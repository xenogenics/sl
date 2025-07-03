(def write (fd val)
  (syscall WRITE fd val))
