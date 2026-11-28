import time,sys
w=time.time();c=time.process_time();t=w+6
while time.time()<t: pass
print(sys.argv[1], round((time.process_time()-c)/(time.time()-w),2))
