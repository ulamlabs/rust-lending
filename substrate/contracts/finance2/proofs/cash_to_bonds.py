import z3

s = z3.Solver()
MAX = 2**128 - 1
cash, total_bonds, bonds, total_liquidity, total_borrowable = z3.Ints('cash total_bonds bonds total_liquidity total_borrowable')

assumptions = []

total_debt = total_liquidity - total_borrowable
assumptions.append(total_debt < 0)
assumptions.append(total_debt > MAX)
s.add(total_liquidity >= 0, total_liquidity <= MAX, total_borrowable >= 0, total_borrowable <= total_liquidity)
s.add(cash >= 0, cash <= MAX)
s.add(total_bonds >= 0, total_bonds <= total_debt)
s.add(bonds >= 0, bonds <= total_bonds)

max_to_burn = z3.Int('max_to_burn')
s.add(max_to_burn == z3.If(total_debt != 0, cash * total_bonds / total_debt, 0))
assumptions.append(max_to_burn > MAX)

to_burn = z3.If(max_to_burn < bonds, max_to_burn, bonds)

repaid = z3.Int('repaid')
s.add(repaid == z3.If(total_bonds != 0, (to_burn*total_debt + total_bonds - 1) / total_bonds, 0))
assumptions.append(repaid > MAX)

new_cash = z3.Int('new_cash')
s.add(new_cash == cash - repaid)
assumptions.append(new_cash < 0)

new_total_borrowable = total_borrowable + repaid
assumptions.append(new_total_borrowable > MAX)

new_bonds = bonds - to_burn
assumptions.append(new_bonds < 0)

new_total_bonds = total_bonds - to_burn
assumptions.append(new_total_bonds < 0)

assumptions.append(new_total_borrowable > total_liquidity)
assumptions.append(new_total_bonds > total_liquidity - new_total_borrowable)
assumptions.append(new_bonds > new_total_bonds)

for a in assumptions:
    t = str(a).replace('\n', ' ').replace('  ', '')
    print('checking...', t, end=' ')
    if s.check(a) == z3.unsat:
        print('OK')
    else:
        m = s.model()
        print('FAILED')
        print('MAX =', MAX)
        print('cash =', m.evaluate(cash))
        print('total_liquidity  =', m.evaluate(total_liquidity))
        print('total_borrowable =', m.evaluate(total_borrowable))
        print('total_bonds      =', m.evaluate(total_bonds))
        print('bonds            =', m.evaluate(bonds))
        print('total_debt =', m.evaluate(total_debt))
        print('new_cash =', m.evaluate(new_cash))
        print('to_burn =', m.evaluate(to_burn))
        print('repaid =', m.evaluate(repaid))
        print('required_cash =', m.evaluate(required_cash))
        print('max_required_cash =', m.evaluate(max_required_cash))
        print('min_required_cash =', m.evaluate(min_required_cash))
        
