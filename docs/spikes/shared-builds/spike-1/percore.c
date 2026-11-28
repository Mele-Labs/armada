#include <mach/mach.h>
#include <stdio.h>
#include <unistd.h>
int main(void){
  double sum=0,cl[3]={0}; natural_t n; processor_info_array_t a,b; mach_msg_type_number_t ca,cb;
  host_processor_info(mach_host_self(),PROCESSOR_CPU_LOAD_INFO,&n,&a,&ca);
  sleep(5);
  host_processor_info(mach_host_self(),PROCESSOR_CPU_LOAD_INFO,&n,&b,&cb);
  for(unsigned i=0;i<n;i++){
    unsigned long t=0,idle;
    for(int s=0;s<CPU_STATE_MAX;s++) t+= b[i*CPU_STATE_MAX+s]-a[i*CPU_STATE_MAX+s];
    idle=b[i*CPU_STATE_MAX+CPU_STATE_IDLE]-a[i*CPU_STATE_MAX+CPU_STATE_IDLE];
    double u=t?100.0*(t-idle)/t:0; sum+=u; cl[i/6]+=u;
  }
  for(unsigned i=0;i<n;i++){} printf("busy %.1f cores  cpu0-5 %.0f%%  cpu6-11 %.0f%%  cpu12-17 %.0f%%\n",sum/100,cl[0]/6,cl[1]/6,cl[2]/6);
  return 0;
}
