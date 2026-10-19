/* REA ev_95950a99fec450d6d3e2c0b833748e81b7ececc607b217ce47815222ab932b9d; IDMMsgHost.exe; decompiler output, not original source. */

undefined4 FUN_004034a0(undefined4 *param_1)

{
  undefined4 *puVar1;
  DWORD DVar2;
  int iVar3;
  undefined4 uVar4;
  code *pcVar5;
  code *pcVar6;
  
  puVar1 = param_1;
  pcVar6 = WaitForSingleObject_exref;
  DVar2 = WaitForSingleObject((HANDLE)param_1[7],0xffffffff);
  pcVar5 = WriteFile_exref;
  while (DVar2 != 0x80) {
    if (puVar1[8] == 0) {
      iVar3 = ReadFile((HANDLE)puVar1[5],(LPVOID)puVar1[2],puVar1[1],(LPDWORD)&param_1,
                       (LPOVERLAPPED)0x0);
      pcVar5 = WriteFile_exref;
    }
    else {
      iVar3 = (*pcVar5)((HANDLE)puVar1[5],(LPVOID)puVar1[2],puVar1[1],&param_1);
      pcVar6 = WaitForSingleObject_exref;
    }
    uVar4 = 0xc0000008;
    if (iVar3 != 0) {
      uVar4 = 0;
    }
    *puVar1 = uVar4;
    puVar1[1] = param_1;
    SetEvent((HANDLE)puVar1[4]);
    DVar2 = (*pcVar6)(puVar1[7],0xffffffff);
  }
  return 0;
}


