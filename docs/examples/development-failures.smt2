; AI observes one development operation. Inputs are facts, not errno guesses.
(declare-datatypes () ((Cause Unclassified Temporary Resource Defect Recurrent)
                       (Effect ReadOnly NotApplied Unknown Applied)
                       (Next Verify Inspect Reconcile Relieve Retry Repair Stop)))

; NotApplied includes a fence on the old request. replaySafe means the receiver
; guarantees atomic deduplication for this identity, input and time window.
(define-fun choose ((cause Cause) (effect Effect) (replaySafe Bool)
                    (capacityRestored Bool) (attempts Int) (inTime Bool)
                    (cancelled Bool)) Next
  (ite cancelled Stop
  (ite (= effect Applied) Verify
  (ite (not inTime) Stop
  (ite (and (= effect Unknown) (not replaySafe)) Reconcile
  (ite (or (= cause Defect) (= cause Recurrent)) Repair
  (ite (= cause Unclassified) Inspect
  (ite (<= attempts 0) Stop
  (ite (and (= cause Resource) (not capacityRestored)) Relieve
       Retry)))))))))

(declare-const cause Cause)
(declare-const effect Effect)
(declare-const replaySafe Bool)
(declare-const capacityRestored Bool)
(declare-const attempts Int)
(declare-const inTime Bool)
(declare-const cancelled Bool)
(define-fun next () Next
  (choose cause effect replaySafe capacityRestored attempts inTime cancelled))

; Ask for an unsafe recommendation over every classification and integer budget.
(push)
(assert (or
  (and (= next Retry)
       (or cancelled (not inTime) (<= attempts 0) (= effect Applied)
           (and (= effect Unknown) (not replaySafe))
           (= cause Unclassified) (= cause Defect) (= cause Recurrent)
           (and (= cause Resource) (not capacityRestored))))
  (and (= cause Unclassified) (= effect ReadOnly) inTime (not cancelled) (not (= next Inspect)))
  (and (or (= cause Defect) (= cause Recurrent)) (= effect NotApplied)
       inTime (not cancelled) (not (= next Repair)))
  (and (= effect Unknown) (not replaySafe) inTime (not cancelled) (not (= next Reconcile)))
  (and cancelled (not (= next Stop)))))
(check-sat)
(pop)

; Witnesses: the policy permits recovery as well as stopping.
(push)
(assert (and
  (= (choose Temporary ReadOnly false false 1 true false) Retry)
  (= (choose Resource NotApplied false false 1 true false) Relieve)
  (= (choose Resource NotApplied false true 1 true false) Retry)
  (= (choose Recurrent NotApplied false true 1 true false) Repair)))
(check-sat)
(pop)

; Independent operations need their own inputs. This pair of choices is
; possible at the same time; actual scheduling and resource claims are external.
(push)
(assert (= (choose Unclassified Unknown false false 1 true false) Reconcile))
(assert (= (choose Temporary ReadOnly false true 1 true false) Retry))
(check-sat)
(pop)

; Counterexample to retry-on-error while resources remain exhausted.
; A task requesting 7 units on a host with 10 and 5 in use still fails.
(push)
(declare-const capacity Int)
(declare-const occupied Int)
(declare-const demand Int)
(assert (= capacity 10))
(assert (= occupied 5))
(assert (= demand 7))
(assert (> (+ occupied demand) capacity))
(assert (= (choose Resource NotApplied false false 1 true false) Relieve))
(define-fun naive ((budget Int)) Next (ite (> budget 0) Retry Stop))
(assert (= (naive 1) Retry))
(check-sat)
(pop)
