/* REA ev_864c4fcc246ee1eaf48477b482d3ef3575e93308db3594f9bad33b0fc4f2d922; IDMMsgHost.exe; decompiler output, not original source. */

void __fastcall FUN_00403530(int *param_1,undefined4 param_2)

{
  char cVar1;
  uint uVar2;
  DWORD dwProcessId;
  HANDLE hObject;
  uint uVar3;
  int iVar4;
  int iVar5;
  char *pcVar6;
  char *pcVar7;
  undefined4 *puVar8;
  undefined1 local_44 [20];
  DWORD local_30;
  undefined1 *local_2c;
  int local_28;
  int local_24;
  char local_20;
  undefined1 local_1f;
  undefined1 local_1a [2];
  DWORD local_18;
  DWORD local_14;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  local_8 = 0xffffffff;
  puStack_c = &LAB_0040396d;
  local_10 = ExceptionList;
  uVar2 = DAT_00406000 ^ (uint)&stack0xfffffffc;
  ExceptionList = &local_10;
  dwProcessId = FUN_00402300();
  if ((DAT_004066ac == (code *)0x0) || (DAT_004066b0 == (code *)0x0)) {
    if ((DAT_004066b4 != (code *)0x0) && (DAT_004066a8 != (code *)0x0)) {
      FUN_00401e30((int *)&local_2c);
      local_1f = 1;
      local_8 = 0;
      hObject = OpenProcess(0x400,0,dwProcessId);
      if (hObject != (HANDLE)0x0) {
        if (local_24 == 0) {
          uVar3 = 0;
        }
        else {
          uVar3 = (local_24 - 1U) / ((local_20 < '\0') + 1);
        }
        uVar2 = (*DAT_004066b4)(hObject,local_2c,uVar3,uVar2);
        FUN_004020a0(&local_2c,uVar2);
        if (-1 < (int)(local_28 - 8U)) {
          uVar2 = FUN_00401fb0(&local_2c,L"\\cmd.exe",8,local_28 - 8U);
          if (uVar2 == 0) {
            iVar4 = (*DAT_004066a8)(hObject,0,local_44,0x18,0);
            if (iVar4 == 0) {
              dwProcessId = local_30;
            }
          }
        }
        CloseHandle(hObject);
      }
      if ((local_2c != local_1a) && (local_2c + -4 != (undefined1 *)0x0)) {
        free(local_2c + -4);
      }
    }
  }
  else {
    local_14 = 0;
    local_18 = 0;
    (*DAT_004066ac)(param_2,&local_18);
    (*DAT_004066b0)(param_2,&local_14);
    if ((dwProcessId == local_14) && (local_18 != 0)) {
      dwProcessId = local_18;
    }
  }
  iVar4 = FUN_00401c70(param_1,4);
  if (iVar4 != 0) {
    param_1[1] = param_1[1] + 4;
  }
  iVar4 = param_1[1];
  if ((uint)param_1[2] < iVar4 + 0xfU) {
    iVar5 = FUN_00401c70(param_1,0xf);
    if (iVar5 != 0) goto LAB_0040369f;
  }
  else {
LAB_0040369f:
    puVar8 = (undefined4 *)(param_1[1] + *param_1);
    *puVar8 = 0x2347534d;
    puVar8[1] = 0x37312330;
    puVar8[2] = 0x30233123;
    *(undefined2 *)(puVar8 + 3) = 0x393a;
    *(undefined1 *)((int)puVar8 + 0xe) = 0x3a;
    param_1[1] = iVar4 + 0xfU;
  }
  iVar4 = 10;
  pcVar6 = (char *)FUN_00401c70(param_1,0xb);
  pcVar7 = _itoa(dwProcessId,pcVar6,iVar4);
  pcVar6 = pcVar7 + 1;
  do {
    cVar1 = *pcVar7;
    pcVar7 = pcVar7 + 1;
  } while (cVar1 != '\0');
  iVar4 = FUN_00401c70(param_1,(int)pcVar7 - (int)pcVar6);
  if (iVar4 != 0) {
    param_1[1] = param_1[1] + ((int)pcVar7 - (int)pcVar6);
  }
  uVar2 = param_1[1] + 1;
  if ((uint)param_1[2] < uVar2) {
    iVar4 = FUN_00401c70(param_1,1);
    if (iVar4 == 0) {
      uVar2 = param_1[1];
      goto LAB_0040372f;
    }
  }
  *(undefined1 *)(param_1[1] + *param_1) = 0x3b;
  param_1[1] = uVar2;
LAB_0040372f:
  *(uint *)*param_1 = uVar2 - 4;
  ExceptionList = local_10;
  return;
}


