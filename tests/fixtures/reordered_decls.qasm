OPENQASM 3.0;
include "stdgates.inc";
bit[3] c;
qubit[3] q;
// comment noise and spacing changes only
h  q[0];
cx q[0],q[1];
cx q[1],q[2];
rz(0.5)   q[2];
barrier q[0], q[1], q[2];
c[0] = measure q[0];
c[1] = measure q[1];
c[2] = measure q[2];
