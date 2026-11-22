n1 --sim-c 33 --sim-b 33 --sim-k 4 --sim-policy random --seed 21
n2 --sim-c 85 --sim-b 340 --sim-k 1 --sim-policy min
n3 --sim-c 15 --sim-b 120 --sim-k 16 --sim-policy mixed --seed 22
n4 --sim-c 55 --sim-b 14 --sim-k 1 --sim-policy random --seed 23 --app onoff:12:6
n5 --sim-c 40 --sim-b 80 --sim-k 4 --sim-policy max --sim-c-schedule 30:80 --app rate:60
n6 --sim-c 65 --sim-b 130 --sim-k 16 --sim-policy random --seed 24 --sim-c-schedule 35:30
n7 --sim-c 25 --sim-b 50 --sim-k 1 --sim-policy mixed --seed 25 --app random --sim-b-schedule 30:15
n8 --sim-c 100 --sim-b 100 --sim-k 4 --sim-policy min
