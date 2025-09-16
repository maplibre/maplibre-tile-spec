#include<immintrin.h>
int main(){
__m256i x=_mm256_set1_epi32(5);
x=_mm256_add_epi32(x,x);
return _mm256_movemask_epi8(x);
}
