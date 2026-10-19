/* REA ev_72edfd2acc00b355adedd60e01c6155a077aca40ecf60dd5456c202d9c02571b; decompiler output, not original source. */

void FUN_005b7ef0(void)

{
  HKEY hKey;
  int in_ECX;
  int local_4;
  
  hKey = *(HKEY *)(in_ECX + 0x244);
  if (hKey != (HKEY)0x0) {
    if ((*(int *)(in_ECX + 0x17c) != 0) && (local_4 = *(int *)(in_ECX + 0x1a8), local_4 != 0)) {
      RegSetValueExA(hKey,"bUseSpeedLimiter",0,4,(BYTE *)&local_4,4);
      RegSetValueExA(*(HKEY *)(in_ECX + 0x244),"psnSpeedLimit",0,0,(BYTE *)(in_ECX + 0x170),8);
      return;
    }
    RegDeleteValueA(hKey,"bUseSpeedLimiter");
    RegDeleteValueA(*(HKEY *)(in_ECX + 0x244),"psnSpeedLimit");
  }
  return;
}


