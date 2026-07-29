import z3
from dataclasses import dataclass
from qe.util import z3_min


class Ideal:

    @dataclass
    class Config:
        T: int = 5
        R: int = 1

        infinite_buffer: bool = False

        use_loss_detect: bool = False

    class Variables:
        def __init__(self, name: str, c):
            self.A = [z3.Real(f'{name}A_{t}') for t in range(c.T)]
            self.S = [z3.Real(f'{name}S_{t}') for t in range(c.T)]
            self.L = [z3.Real(f'{name}L_{t}') for t in range(c.T)]
            self.I = [z3.Real(f'{name}I_{t}') for t in range(c.T)]  # service curve of ideal link
            self.L0 = z3.Real(f'{name}L0')
            self.C = z3.Real(f'{name}C')
            self.B = z3.Real(f'{name}B')  # buffer size
            self.a = z3.Real(f'{name}a')  # pkt size

            if c.use_loss_detect:
                self.Ld = [z3.Real(f'{name}Ld_{t}') for t in range(c.T)]

    class Constraints:
        def __init__(self, c, v):
            self.c = c
            self.v = v

        def non_det_initial(self):
            c, v = self.c, self.v

            cl = []
            cl.append(v.L[0] >= 0)
            cl.append(v.L[0] >= v.L0)
            cl.append(v.L0 >= 0)
            if c.use_loss_detect:
                cl.append(v.Ld[0] >= 0)
            cl.append(v.I[0] <= v.A[0] - v.L[0])
            cl.append(v.C >= 5 * v.a)  # ~BDP is at least 5 pkts.
            cl.append(v.B >= 5 * v.a)
            cl.append(v.a == 1)
            return cl

        def det_loss(self):
            c, v = self.c, self.v

            cl = []
            if c.infinite_buffer:
                for t in range(c.T):
                    cl.append(v.L[t] == 0)
                return cl

            for t in range(1, c.T):
                cl.append(
                    z3.Implies(
                        v.A[t] - v.L[t - 1] - v.I[t] > v.B,
                        v.L[t] == v.A[t] - v.I[t] - v.B,
                    )
                )
                cl.append(
                    z3.Implies(
                        z3.Not(v.A[t] - v.L[t - 1] - v.I[t] > v.B),
                        v.L[t] == v.L[t - 1],
                    )
                )
            return cl

        def oracle_loss_detect(self):
            c, v = self.c, self.v

            cl = []
            for t in range(c.R, c.T):
                cl.append(v.Ld[t] == v.L[t - c.R])
            return cl

        def loss_detect(self):
            # NOTE: In the CCAC code, the Ld variable also has a t-c.R and so
            # they use Ld[t] to compute A[t]. In the convention below,
            # Ld[t-c.R] would be visible when computing A[t].

            c, v = self.c, self.v

            cl = []
            for t in range(1, c.T):
                cl.append(v.Ld[t] >= v.Ld[t - 1])

            for t in range(c.T):
                for dt in range(t + 1):
                    detectable = v.S[t] >= v.A[t - dt] - v.L[t - dt] + v.a
                    cl.append(z3.Implies(detectable, v.Ld[t] >= v.L[t - dt]))
                    cl.append(z3.Implies(z3.Not(detectable), v.Ld[t] <= v.L[t - dt]))
                if t - 1 >= 0:
                    cl.append(v.Ld[t - 1] <= v.Ld[t])
                cl.append(v.Ld[t] <= v.L[t])
            return cl

        def det_service_ideal(self):
            c, v = self.c, self.v

            cl = []
            for t in range(1, c.T):
                cl.append(v.I[t] == z3_min(v.I[t - 1] + v.C * 1, v.A[t] - v.L[t]))
            return cl

        def non_det_cca(self):
            c, v = self.c, self.v

            cl = []
            for t in range(1, c.T):
                cl.append(v.A[t] >= v.A[t - 1] + v.a)

            return cl

        def non_dec_arrival(self):
            c, v = self.c, self.v

            cl = []
            for t in range(1, c.T):
                cl.append(v.A[t] >= v.A[t - 1])

            return cl

    def __init__(self, name: str = '', c: Config = None):
        self.c = c if c is not None else self.Config()
        self.v = self.Variables(name, self.c)
        self.constraints = self.Constraints(self.c, self.v)

    def all_constraints(self):
        ret = (
            self.constraints.non_det_initial()
            + self.constraints.det_loss()
            + self.constraints.det_service_ideal()
            + self.constraints.non_dec_arrival()
        )
        if self.c.use_loss_detect:
            ret += self.constraints.loss_detect()
        return ret
