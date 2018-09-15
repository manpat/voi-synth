(gain 0.2)
(def-store osc-feedback)

(let pulse (sqr 3))
(let sequence [55 (* 55 2) (* 55 3) (* 55 4) (* 55 5) (* 55 6)])

(let freq (sequencer sequence pulse))

(let fm (sin (+ (* 2 freq) (* (env-ar 0.01 0.5 pulse) freq osc-feedback))))
(store osc-feedback fm)
; (let fm (sin (+ (* 3 freq) (* 55 fm))))

(let freq (+ freq (* 110 fm)))

(let osc (+
	(sqr (* freq 0.5))
	(sqr (* freq 1))
	(sqr (* freq 2))
	(sqr (* freq 3))
))


(* (env-ar 0.01 0.5 pulse) osc)