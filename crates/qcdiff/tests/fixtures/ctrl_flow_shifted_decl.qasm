OPENQASM 3.0;
include "stdgates.inc";
int extra = 1;
qubit[2] q;
bit[2] c;
if (c[0]) {
  x q[1];
}
