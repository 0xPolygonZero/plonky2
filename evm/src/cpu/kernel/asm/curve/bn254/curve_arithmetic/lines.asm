/// p1, p2 : [Fp; 2], q : [Fp2; 2]

/// def tangent(px, py, qx, qy):
///     return
///         py**2 - 9, 
///         (-3*px**2) * qx, 
///         (2*py)     * qy,

%macro tangent
%endmacro

/// def cord(p1x, p1y, p2x, p2y, qx, qy):
///     return
///         p1y*p2x - p2y*p1x, 
///         (p2y - p1y) * qx, 
///         (p1x - p2x) * qy,

%macro cord
%endmacro