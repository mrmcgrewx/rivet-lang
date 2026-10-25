/* Generic scalar output contract, independent of any algorithm. */
#include "../../native/runtime.c"
#include <float.h>

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    double value;
    if (strcmp(argv[1], "half") == 0) value = 2.5;
    else if (strcmp(argv[1], "negative-zero") == 0) value = -0.0;
    else if (strcmp(argv[1], "max") == 0) value = DBL_MAX;
    else if (strcmp(argv[1], "subnormal") == 0) value = 0x0.0000000000001p-1022;
    else if (strcmp(argv[1], "infinity") == 0) value = INFINITY;
    else if (strcmp(argv[1], "nan") == 0) value = NAN;
    else return 2;
    rv2_return_float(value);
    rv_cleanup();
    return 0;
}
