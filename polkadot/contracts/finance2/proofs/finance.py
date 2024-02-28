import z3

s = z3.Solver()
MAX = 255
DEPOSIT = 0
WITHDRAW = 1
MINT = 2
BURN = 3
BORROW = 4
REPAY = 5
LIQUIDATE = 6

assumptions = []

total_collateral0, collateral0, last_total_liquidity0, total_shares0, shares0, total_borrowable0, total_bonds0, bonds0 = z3.Ints('total_collateral0 collateral0 last_total_liquidity0 total_shares0 shares0 total_borrowable0 total_bonds0 bonds0')
s.add(total_collateral0 == 0, collateral0 == 0, last_total_liquidity0 == 0, total_shares0 == 0, shares0 == 0, total_borrowable0 == 0, total_bonds0 == 0, bonds0 == 0)

total_collateral1, collateral1, last_total_liquidity1, total_shares1, shares1, total_borrowable1, total_bonds1, bonds1, x0, action0, interest0 = z3.Ints('total_collateral1 collateral1 last_total_liquidity1 total_shares1 shares1 total_borrowable1 total_bonds1 bonds1 x0 action0 interest0')
s.add(x0 >= 0, x0 <= MAX)
s.add(interest0 >= 0, interest0 <= MAX)

total_liquidity1 = last_total_liquidity0 + interest0
s.add(total_liquidity1 <= MAX)

collateral_delta1 = z3.If(action0 == DEPOSIT, x0, z3.If(action0 == WITHDRAW, -x0, 0))

s.add(total_collateral1 == total_collateral0 + collateral_delta1)
s.add(z3.Implies(action0 == DEPOSIT, total_collateral1 <= MAX))
assumptions.append(total_collateral1 < 0)

s.add(collateral1 == collateral0 + collateral_delta1)
s.add(z3.Implies(action0 == WITHDRAW, collateral1 >= 0))
assumptions.append(collateral1 > MAX)

last_shares1 = shares0
s.add(z3.Implies(action0 == BURN, x0 <= last_shares1))

liquidity_from_shares1 = z3.If(action0 == BURN, z3.If(total_shares0 == 0, 0, x0 * total_liquidity1 / total_shares0), 0)
assumptions.append(liquidity_from_shares1 > MAX)

liquidity_delta1 = z3.If(action0 == MINT, x0, -liquidity_from_shares1)
s.add(last_total_liquidity1 == total_liquidity1 + liquidity_delta1)

s.add(z3.Implies(action0 == MINT, last_total_liquidity1 <= MAX))
assumptions.append(last_total_liquidity1 < 0)

shares_from_liquidity1 = z3.If(action0 == MINT, z3.If(total_liquidity1 == 0, x0, x0 * total_shares0 / total_liquidity1), 0)
assumptions.append(shares_from_liquidity1 > MAX)

share_delta1 = z3.If(action0 == BURN, -x0, shares_from_liquidity1)
s.add(total_shares1 == total_shares0 + share_delta1)
assumptions.append(total_shares1 < 0)
assumptions.append(total_shares1 > MAX)

s.add(shares1 == last_shares1 + share_delta1)
assumptions.append(shares1 < 0)
assumptions.append(shares1 > MAX)

total_debt1 = total_liquidity1 - total_borrowable0
bonds_from_debt1 = z3.If(action0 == BORROW, z3.If(total_debt1 == 0, x0, (x0 * total_bonds0 + total_debt1 - 1) / total_debt1), 0)
assumptions.append(bonds_from_debt1 > MAX)

bonds_delta1 = z3.If(action0 == BORROW, bonds_from_debt1, -x0)
s.add(total_bonds1 == total_bonds0 + bonds_delta1)

last_bonds1 = bonds0
s.add(bonds1 == last_bonds1 + bonds_delta1)
s.add(z3.Implies(action0 == REPAY, bonds1 >= 0))

borrowable_from_bonds1 = z3.If(action0 == REPAY, z3.If(total_bonds0 == 0, total_debt1, (x0 * total_debt1 + total_bonds0 - 1) / total_bonds0), 0)
borrowable_delta1 = z3.If(action0 == BORROW, -x0, z3.If(action0 == REPAY, borrowable_from_bonds1, liquidity_delta1))
s.add(total_borrowable1 == total_borrowable0 + borrowable_delta1)
s.add(z3.Implies(action0 == BORROW, total_borrowable1 >= 0))
s.add(z3.Implies(action0 == BURN, total_borrowable1 >= 0))
assumptions.append(total_borrowable1 > MAX)
assumptions.append(total_borrowable1 > total_liquidity1)
assumptions.append(total_borrowable1 > last_total_liquidity1)

if s.check(z3.Or(*assumptions)) == z3.unsat:
    print("OK")
else:
    m = s.model()
    print(m)