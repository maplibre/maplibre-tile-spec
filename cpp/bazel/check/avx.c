#include<immintrin.h>
int main(){
__m128 x=_mm_set1_ps(0.5);
x=_mm_permute_ps(x,1);
return _mm_movemask_ps(x);
}
