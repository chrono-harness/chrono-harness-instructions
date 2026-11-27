; One logical operation, one input and one receiver. Check with z3 recovery.smt2.
(declare-datatypes () ((Phase Ready Uncertain Done Defect Stopped)
                       (Reply Confirmed NotApplied LostBefore LostAfter Permanent)
                       (Action Attempt ObserveCommitted ObserveAbsent Stop)
                       (State (state (phase Phase) (remaining Int) (effects Int) (firstError Int)))))

; A strictly positive error identity stands for an original cause retained by the caller.
(define-fun remember ((s State) (e Int)) Int
  (ite (= (firstError s) 0) e (firstError s)))

; A receiver promising atomic deduplication applies the same identity at most once.
(define-fun commit ((s State) (dedup Bool)) Int
  (ite (and dedup (> (effects s) 0)) (effects s) (+ 1 (effects s))))

(define-fun attempt ((s State) (r Reply) (dedup Bool) (err Int)) State
  (state (ite (= r Confirmed) Done
         (ite (= r Permanent) Defect
         (ite (= r NotApplied) (phase s) Uncertain)))
         (- (remaining s) 1)
         (ite (or (= r Confirmed) (= r LostAfter)) (commit s dedup) (effects s))
         (ite (= r Confirmed) (firstError s) (remember s err))))

; An authoritative negative observation includes a fence: an earlier request can
; no longer commit. A missing receipt or stale read cannot set this input.
(define-fun allowed ((s State) (a Action) (dedup Bool) (windowOpen Bool)
                     (canProceed Bool) (fenced Bool)) Bool
  (match a
    ((Attempt (and (> (remaining s) 0) windowOpen canProceed
                   (or (= (phase s) Ready) (and (= (phase s) Uncertain) dedup))))
     (ObserveCommitted (and (= (phase s) Uncertain) (= (effects s) 1)))
     (ObserveAbsent (and (= (phase s) Uncertain) fenced (= (effects s) 0)))
     (Stop (or (= (phase s) Ready) (= (phase s) Uncertain) (= (phase s) Defect))))))

(define-fun step ((s State) (a Action) (r Reply) (dedup Bool) (err Int)) State
  (ite (= a Attempt) (attempt s r dedup err)
       (state (ite (= a ObserveCommitted) Done (ite (= a ObserveAbsent) Ready Stopped))
              (remaining s) (effects s) (firstError s))))

(define-fun invariant ((s State) (limit Int)) Bool
  (and (<= 0 (remaining s) limit) (<= 0 (effects s) 1) (>= (firstError s) 0)
       (=> (= (phase s) Ready) (= (effects s) 0))
       (=> (= (phase s) Done) (= (effects s) 1))
       (=> (or (= (phase s) Uncertain) (= (phase s) Defect)) (> (firstError s) 0))))

(declare-const s State)
(declare-const a Action)
(declare-const r Reply)
(declare-const limit Int)
(declare-const dedup Bool)
(declare-const windowOpen Bool)
(declare-const canProceed Bool)
(declare-const fenced Bool)
(declare-const err Int)
(define-fun after () State (step s a r dedup err))
(assert (>= limit 0))
(assert (> err 0))

; Initial state and induction for arbitrary finite budgets.
(push)
(assert (not (invariant (state Ready limit 0 0) limit)))
(check-sat)
(pop)
(push)
(assert (invariant s limit))
(assert (allowed s a dedup windowOpen canProceed fenced))
(assert (not (and (invariant after limit)
                  (= (remaining after) (- (remaining s) (ite (= a Attempt) 1 0)))
                  (=> (> (firstError s) 0) (= (firstError after) (firstError s))))))
(check-sat)
(pop)

; A safety guard must still allow progress. Lose a receipt, observe the
; completed write, and finish with one effect and the original failure.
(define-fun lost () State (attempt (state Ready 2 0 0) LostAfter false 17))
(push)
(assert (allowed lost ObserveCommitted false true true false))
(assert (= (step lost ObserveCommitted Confirmed false 23) (state Done 1 1 17)))
(check-sat)
(pop)

; Mutant: bypass the guard and replay the same non-idempotent write blindly.
(push)
(assert (not (allowed lost Attempt false true true false)))
(assert (= (effects (attempt lost Confirmed false 23)) 2))
(check-sat)
(pop)
