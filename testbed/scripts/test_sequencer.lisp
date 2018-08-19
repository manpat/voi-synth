(gain 0.2)
(def-store osc-feedback)

(let pulse (sqr 4))
(let sequence [55 (* 55 2) (* 55 3) (* 55 4) (* 55 5) (* 55 6)])

(let freq (sequencer sequence pulse))

(let fm (sin (+ (* 2 freq) (* 55 osc-feedback))))
(let fm (sin (+ (* 3 freq) (* fm 55))))

(let osc (sqr (+ freq (* fm 110))))
(store osc-feedback osc)

(* (env-ar 0.01 0.3 pulse) osc)