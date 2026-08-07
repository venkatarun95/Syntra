import z3
from dataclasses import dataclass
from qe.environment.ideal import Ideal


class BurstyCBRDelay(Ideal):

    @dataclass
    class Config(Ideal.Config):
        D: int = 1
        use_qdel: bool = False
        use_hop_delay: bool = False

    class Variables(Ideal.Variables):
        def __init__(self, name: str, c):
            super().__init__(name, c)
            # self.S = z3.RealVarVector(c.T)

            # Maximum burstiness of the sender in bytes
            self.K = z3.Real(f'{name}K')
            # Packets after initial perturbation from bursty sending
            self.P = [z3.Real(f'{name}P_{t}') for t in range(c.T)]  # service curve of ideal link
            # I think using RealVarVector just makes those vars have a forall quantifier.
            self.I = [z3.Real(f'{name}I_{t}') for t in range(c.T)]  # service curve of ideal link

            if c.use_qdel:
                self.qdel = [
                    [z3.Bool(f"{name}qdel_{t},{dt}") for dt in range(c.T)]
                    for t in range(c.T)
                ]
                # Since CCA only sees Ld, qdel cannot be reconstructed from
                # just A, Ld, S. qdel actually leaks information about L to
                # CCA.
            if c.use_hop_delay:
                self.hop_delay = [
                    [z3.Bool(f"{name}hop_delay_{t},{dt}") for dt in range(c.T)]
                    for t in range(c.T)
                ]

    class Constraints(Ideal.Constraints):
        def hop_delay(self):
            # hop_delay[t][dt]: True iff the bottleneck queueing delay at time t is
            # greater than or equal to dt.

            c, v = self.c, self.v

            if not c.use_hop_delay:
                return []

            cl = []
            for t in range(c.T):
                for dt in range(t+1):
                    cl.append(
                        v.hop_delay[t][dt] == (v.A[t-dt] - v.L[t-dt] >= v.I[t])
                    )

            return cl

        def perturb(self):
            c, v = self.c, self.v

            cl = []
            cl.append(v.K > 0)
            for t in range(0, c.T):
                cl.append(v.P[t] >= v.A[t] - v.K)
                cl.append(v.P[t] <= v.A[t])
            return cl

        # Override this to work off of P
        def det_service_ideal(self):
            c, v = self.c, self.v

            cl = []
            for t in range(1, c.T):
                cl.append(v.I[t] == z3_min(v.I[t - 1] + v.C * 1, v.P[t] - v.L[t]))
            return cl


        def non_det_service(self):
            c, v = self.c, self.v

            cl = []
            # cl.append(v.S[0] >= 0)
            for t in range(c.T):
                if t - c.D >= 0:
                    cl.append(v.S[t] >= v.I[t - c.D])
                else:
                    assert c.D == 1
                    assert t == 0
                    cl.append(v.S[t] >= v.I[t] - v.C * c.D)
                if t - 1 >= 0:
                    cl.append(v.S[t] >= v.S[t - 1])
                cl.append(v.S[t] <= v.I[t])
            return cl

        def calculate_qdel_defs(self):
            # qdel[t][dt>=t] is non-deterministic (including,
            #                qdel[0][dt], qdel[t][dt>t-1])
            # qdel[t][dt<t] is deterministic
            """
                dt
                0 1 2 3
            ----------
            0| n n n n
            t 1| d n n n
            2| d d n n
            3| d d d n
            """

            c, v = self.c, self.v

            cl = []
            for t in range(1, c.T):
                for dt in range(t):
                    cl.append(
                        z3.Implies(
                            v.S[t] != v.S[t - 1],
                            v.qdel[t][dt]
                            == z3.And(
                                v.A[t - dt - 1] - v.L[t - dt - 1] < v.S[t],
                                v.A[t - dt] - v.L[t - dt] >= v.S[t],
                            ),
                        )
                    )
                    cl.append(
                        z3.Implies(
                            v.S[t] == v.S[t - 1], v.qdel[t][dt] == v.qdel[t - 1][dt]
                        )
                    )

            return cl

        def calculate_qdel_env(self):
            c, v = self.c, self.v

            cl = []
            # There can be only one value for queuing delay at a given time.
            # Needed only for non-deterministic choices, mostly a sanity
            # constraint for deterministic variables.
            for t in range(c.T):
                cl.append(z3.Sum(*v.qdel[t]) <= 1)

            # Let solver choose non-deterministically what happens for t = 0,
            # i.e., no constraint on qdel[0][dt].
            for t in range(1, c.T):
                for dt in range(t, c.T):
                    cl.append(
                        z3.Implies(
                            v.S[t] == v.S[t - 1], v.qdel[t][dt] == v.qdel[t - 1][dt]
                        )
                    )
                    # We let solver choose non-deterministically what happens
                    # when S[t] != S[t-1] for dt > t-1, i.e., no constraint on
                    # qdel[t][dt>t-1]

            # qdel[t][dt] is True iff queueing delay is >=dt but <dt+1 If
            # queuing delay at time t1 is dt1, then queuing delay at time
            # t2=t1+1, cannot be more than dt1+1. I.e., qdel[t2][dt1+1+1] has
            # to be false. Note qdel[t2][dt1+1] can be true as queueing delay
            # at t1+1 can be dt1+1.
            for t1 in range(c.T - 1):
                for dt1 in range(c.T):
                    t2 = t1 + 1
                    # dt2 starts from dt1+1+1
                    cl.append(
                        z3.Implies(
                            v.qdel[t1][dt1],
                            z3.And(
                                *[
                                    z3.Not(v.qdel[t2][dt2])
                                    for dt2 in range(dt1 + 1 + 1, c.T)
                                ]
                            ),
                        )
                    )
                    cl.append(
                        z3.Implies(
                            v.qdel[t1][dt1],
                            z3.Or(
                                *[
                                    v.qdel[t2][dt2]
                                    for dt2 in range(min(c.T, dt1 + 1 + 1))
                                ]
                            ),
                        )
                    )

            # Allow Qdel for time < 0 to be larger than maxdelay as the link
            # rate could have changed.
            # # Delay cannot be more than max_delay. This means that one of
            # # qdel[t][dt<=max_delay] must be true. This also means that all of
            # # qdel[t][dt>max_delay] are false.
            # max_delay = v.B/v.C + c.D
            # for t in range(c.T):
            #     some_qdel_is_true = (1 == z3.Sum(
            #         *[z3.If(dt <= max_delay, v.qdel[t][dt], False)
            #         for dt in range(c.T)]))
            #     # if max_delay is very high, then all qdel can be false.
            #     cl.append(z3.Implies(max_delay <= c.T-1, some_qdel_is_true))

            return cl

    def __init__(self, name: str = '', c: Config = None):
        self.c = c if c is not None else self.Config()
        self.v = self.Variables(name, self.c)
        self.constraints = self.Constraints(self.c, self.v)

    def all_constraints(self):
        c = self.c

        ret = (
            self.constraints.non_det_initial()
            + self.constraints.det_loss()
            + self.constraints.perturb()
            + self.constraints.det_service_ideal()
            + self.constraints.non_det_service()
            + self.constraints.non_dec_arrival()
        )
        if c.use_loss_detect:
            ret += self.constraints.loss_detect()
        if c.use_qdel:
            ret += self.constraints.calculate_qdel_defs() + self.constraints.calculate_qdel_env()
        if c.use_hop_delay:
            ret += self.constraints.hop_delay()
        return ret
