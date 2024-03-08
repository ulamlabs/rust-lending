import z3

s = z3.Solver()
MAX = 256
total_coll0, user_coll0 = z3.Ints('total_coll0 user_coll0')
s.add(total_coll0 == 0, user_coll0 == 0)

total_coll1, user_coll1, amount0 = z3.Ints('total_coll1 user_coll1 amount0')
s.add(total_coll1 == total_coll0 + amount0)
s.add(amount0 < MAX, amount0 > -MAX)
s.add(z3.If(amount0 < 0, user_coll0 + amount0 >= 0, total_coll0 + amount0 < MAX))

s.add(user_coll1 == user_coll0 + amount0)

total_coll2, user_coll2, amount1, b0 = z3.Ints('total_coll2 user_coll2 amount1 b0')
s.add(total_coll2 == total_coll1 + amount1)
s.add(amount1 < MAX, amount1 > -MAX)
prev = z3.If(b0 == 1, user_coll1, user_coll0)
s.add(z3.If(amount1 < 0, prev + amount1 >= 0, total_coll1 + amount1 < MAX))

s.add(user_coll2 == prev + amount1)

total_coll3, user_coll3, amount2, b1 = z3.Ints('total_coll3 user_coll3 amount2 b1')
s.add(total_coll3 == total_coll2 + amount2)
s.add(amount2 < MAX, amount2 > -MAX, z3.Distinct(b1, b0))
prev = z3.If(b1 == 2, user_coll2, z3.If(b1 == 1, user_coll1, user_coll0))
s.add(z3.If(amount2 < 0, prev + amount2 >= 0, total_coll3 + amount2 < MAX))

s.add(user_coll3 == prev + amount2)

total_coll4, user_coll4, amount3, b2 = z3.Ints('total_coll4 user_coll4 amount3 b2')
s.add(total_coll4 == total_coll3 + amount3)
s.add(amount3 < MAX, amount3 > -MAX, z3.Distinct(b2, b1, b0))
prev = z3.If(b2 == 3, user_coll3, z3.If(b2 == 2, user_coll2, z3.If(b2 == 1, user_coll1, user_coll0)))
s.add(z3.If(amount3 < 0, prev + amount3 >= 0, total_coll4 + amount3 < MAX))

s.add(user_coll4 == prev + amount3)

total_coll5, user_coll5, amount4, b3 = z3.Ints('total_coll5 user_coll5 amount4 b3')
# Our goal is to prove that user and total collateral are always within the bounds
# To do it, we have to be sure, there is no model for following condition
to_prove = z3.Or(user_coll5 >= MAX, user_coll5 < 0, total_coll5 >= MAX, total_coll5 < 0)

# total collateral is changing depending on the amount
# Previous total collateral does not depend on any parameter
s.add(total_coll5 == total_coll4 + amount4)

# If amount is less than zero, then user withdraws, deposits otherwise
# This is just our model, on chain we use unsigned integers, and handling two cases in separate messages
s.add(amount4 < MAX, amount4 > -MAX)
# b* has to be distinct, otherwise we would assume that user could double spend
s.add(z3.Distinct(b3, b2, b1, b0))
# Previous user collateral depends on, who is sending a message
# user_coll0 is used as a default value, because we allow to double spend ZERO tokens
prev = z3.If(b3 == 4, user_coll4, z3.If(b3 == 3, user_coll3, z3.If(b3 == 2, user_coll2, z3.If(b3 == 1, user_coll1, user_coll0))))

# If user withdraws, then only user collateral overflow need to be checked
withdraw_check = prev + amount4 >= 0
# If user deposits, then only total collateral overflow need to be checked
deposit_check = total_coll5 + amount4 < MAX
is_withdraw = amount4 < 0
# In the code, we do it in few places in code, so it is important choose the right check depending on direction of changes
# Anytime collateral is changed, total and user collateral must be changed by the same amount
mandatory_check = z3.If(is_withdraw, withdraw_check, deposit_check)

# User collateral is changing depending on the amount
# However, previous user collateral depends on, who is sending a message
s.add(user_coll5 == prev + amount4)

#You can set breakpoint here and check, if you just add withdraw_check or deposit_check or none of them
s.add(mandatory_check)

if s.check(to_prove) != z3.unsat:
    m = s.model()
    print(m, 'FAILED')
else:
    s.check()
    print(s.model(), 'PASSED')