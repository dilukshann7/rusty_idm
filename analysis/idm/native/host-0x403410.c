/* REA ev_fff5b25e5cb2290d9da0e3aa5e0960f2107d0b777eed1fb75fc046748181df38; IDMMsgHost.exe; decompiler output, not original source. */

BOOL __fastcall
FUN_00403410(HANDLE param_1,LPCVOID param_2,DWORD param_3,LPDWORD param_4,LPOVERLAPPED param_5,
            DWORD param_6)

{
  HANDLE hEvent;
  BOOL BVar1;
  
  if ((param_5 != (LPOVERLAPPED)0x0) && (param_5[1].InternalHigh != 0)) {
    if (param_4 != (LPDWORD)0x0) {
      *param_4 = 0;
    }
    hEvent = param_5->hEvent;
    param_5[1].u.s.OffsetHigh = param_6;
    param_5[1].Internal = (ULONG_PTR)param_1;
    (param_5->u).Pointer = param_2;
    param_5->Internal = 0x103;
    param_5->InternalHigh = param_3;
    BVar1 = ResetEvent(hEvent);
    if (BVar1 != 0) {
      BVar1 = ReleaseSemaphore(param_5[1].u.Pointer,1,(LPLONG)0x0);
      if (BVar1 != 0) {
        SetLastError(0x3e5);
      }
    }
    return 0;
  }
  if (param_6 != 0) {
    BVar1 = WriteFile(param_1,param_2,param_3,param_4,param_5);
    return BVar1;
  }
  BVar1 = ReadFile(param_1,param_2,param_3,param_4,param_5);
  return BVar1;
}


