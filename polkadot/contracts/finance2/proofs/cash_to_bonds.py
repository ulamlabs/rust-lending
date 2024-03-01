import z3

s = z3.Solver()
MAX = 2**128 - 1
cash, total_bonds, bonds, total_debt = z3.Ints('cash total_bonds bonds total_debt')

assumptions = []

s.add(cash >= 0, cash <= MAX)
s.add(total_debt <= MAX)
s.add(total_bonds >= 0, total_bonds <= total_debt)
s.add(bonds >= 0, bonds <= total_bonds)

to_repay = z3.If(total_debt != 0, cash * total_bonds / total_debt, 0)
assumptions.append(to_repay > MAX)

to_compensate = z3.If(to_repay > bonds, to_repay - bonds, 0)
assumptions.append(to_compensate > MAX)
assumptions.append(to_compensate < 0)

repaid = to_repay - to_compensate
assumptions.append(repaid < 0)

new_bonds = bonds - repaid
assumptions.append(new_bonds < 0)

new_cash = z3.If(total_bonds != 0, to_compensate * total_debt / total_bonds, cash)
assumptions.append(new_cash > MAX)

cash_spent = cash - new_cash
assumptions.append(cash_spent < 0)

min_required_cash = z3.If(total_bonds != 0, repaid * total_debt / total_bonds, 0)
required_cash = z3.If(total_bonds != 0, (repaid * total_debt + total_bonds - 1) / total_bonds, 0)
max_required_cash = z3.If(total_bonds != 0, ((repaid+1) * total_debt + total_bonds - 1) / total_bonds, 0)
assumptions.append(required_cash > MAX)
assumptions.append(required_cash > cash_spent)
assumptions.append(cash_spent > max_required_cash)
assumptions.append(z3.And(cash_spent == min_required_cash, cash_spent > 0, bonds < total_bonds, total_bonds < total_debt, total_bonds * 2 > total_debt, repaid < bonds)) # it should be sat

for a in assumptions:
    t = str(a).replace('\n', ' ').replace('  ', '')
    print('checking...', t, end=' ')
    if s.check(a) == z3.unsat:
        print('OK')
    else:
        m = s.model()
        print('FAILED')
        print('cash =', m.evaluate(cash))
        print('total_bonds =', m.evaluate(total_bonds))
        print('bonds =', m.evaluate(bonds))
        print('total_debt =', m.evaluate(total_debt))
        print('to_repay =', m.evaluate(to_repay))
        print('to_compensate =', m.evaluate(to_compensate))
        print('new_cash =', m.evaluate(new_cash))
        print('cash_spent =', m.evaluate(cash_spent))
        print('repaid =', m.evaluate(repaid))
        print('required_cash =', m.evaluate(required_cash))
        print('max_required_cash =', m.evaluate(max_required_cash))
        print('min_required_cash =', m.evaluate(min_required_cash))
        
