/* REA ev_390e407cd3fdc26bc456280b61d084916ce3d4b2da032ddcfd41170281ecc730; IDMMsgHost.exe; decompiler output, not original source. */

uint __fastcall FUN_00402420(byte *param_1,uint param_2,uint param_3)

{
  byte bVar1;
  byte bVar2;
  int iVar3;
  byte *pbVar4;
  byte *pbVar5;
  byte *pbVar6;
  byte *pbVar7;
  byte *pbVar8;
  uint local_c;
  
  local_c = 0;
  pbVar7 = param_1;
  if (param_1 < param_1 + param_2) {
    do {
      if ((param_3 != 0) && (param_3 <= local_c)) goto LAB_00402489;
      bVar1 = *pbVar7;
      pbVar7 = pbVar7 + 1;
      if ("uuuuuuuubtnufruuuuuuuuuuuuuuuuuu"[bVar1] == '\0') {
        iVar3 = 1;
      }
      else {
        iVar3 = (uint)("uuuuuuuubtnufruuuuuuuuuuuuuuuuuu"[bVar1] == 'u') * 4 + 2;
      }
      local_c = local_c + iVar3;
    } while (pbVar7 < param_1 + param_2);
  }
  if (param_3 != 0) {
LAB_00402489:
    if ((local_c != param_2) && (pbVar4 = param_1 + local_c, param_1 < pbVar7)) {
      pbVar8 = pbVar4 + -2;
      pbVar6 = pbVar4 + -6;
      do {
        if (pbVar4 <= pbVar7) {
          return local_c;
        }
        bVar1 = pbVar7[-1];
        pbVar7 = pbVar7 + -1;
        bVar2 = "uuuuuuuubtnufruuuuuuuuuuuuuuuuuu"[bVar1];
        if (bVar2 == 0) {
          pbVar5 = pbVar4 + -1;
          pbVar6 = pbVar6 + -1;
          pbVar8 = pbVar8 + -1;
          *pbVar5 = bVar1;
        }
        else if (bVar2 == 0x75) {
          if (pbVar6 < param_1) {
            return local_c;
          }
          pbVar6 = pbVar6 + -6;
          pbVar8 = pbVar8 + -6;
          pbVar4[-5] = 0x75;
          pbVar4[-0xffffffff00000004] = 0x30;
          pbVar4[-0xffffffff00000003] = 0x30;
          pbVar5 = pbVar4 + -6;
          *pbVar5 = 0x5c;
          bVar2 = "0123456789abcdef"[(int)(char)bVar1 >> 4];
          pbVar4[-1] = "0123456789abcdef"[(int)(char)bVar1 & 0xf];
          pbVar4[-2] = bVar2;
        }
        else {
          if (pbVar8 < param_1) {
            return local_c;
          }
          pbVar5 = pbVar4 + -2;
          pbVar6 = pbVar6 + -2;
          pbVar8 = pbVar8 + -2;
          pbVar4[-1] = bVar2;
          *pbVar5 = 0x5c;
        }
        pbVar4 = pbVar5;
      } while (param_1 < pbVar7);
    }
  }
  return local_c;
}


