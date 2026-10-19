/* REA ev_929465caf2dad0f41caeed7882c2f75da87d1b26a2f007f3a3af096bcffdb6cc; IDMMsgHost.exe; decompiler output, not original source. */

int __fastcall FUN_00402550(byte *param_1,int param_2,byte *param_3,byte *param_4)

{
  byte bVar1;
  byte *pbVar2;
  int iVar3;
  byte *pbVar4;
  byte *pbVar5;
  byte *pbVar6;
  bool bVar7;
  
  param_4 = param_4 + (int)param_3;
  bVar7 = param_1 == param_3;
  pbVar4 = param_1;
  pbVar5 = param_3;
  if (param_3 < param_4) {
    while (pbVar4 < param_1 + param_2) {
      bVar1 = *pbVar5;
      pbVar6 = pbVar5 + 1;
      if (bVar1 == 0) {
        if (!bVar7) {
          *pbVar4 = 0;
        }
        pbVar4 = pbVar4 + 1;
        break;
      }
      if (bVar1 == 0x5c) {
        param_3 = (byte *)(int)(char)*pbVar6;
        iVar3 = 1;
        pbVar2 = param_3;
        switch(param_3) {
        case (byte *)0x30:
        case (byte *)0x31:
        case (byte *)0x32:
        case (byte *)0x33:
          iVar3 = FUN_00402720(pbVar6,(int *)&param_3,3,'\b');
          pbVar2 = param_3;
          if (iVar3 == 0) {
            pbVar2 = (byte *)0x0;
          }
          break;
        case (byte *)0x61:
          pbVar2 = (byte *)0x7;
          break;
        case (byte *)0x62:
          pbVar2 = (byte *)0x8;
          break;
        case (byte *)0x66:
          pbVar2 = (byte *)0xc;
          break;
        case (byte *)0x6e:
          pbVar2 = (byte *)0xa;
          break;
        case (byte *)0x72:
          pbVar2 = (byte *)0xd;
          break;
        case (byte *)0x74:
          pbVar2 = (byte *)0x9;
          break;
        case (byte *)0x75:
          pbVar6 = pbVar5 + 2;
          iVar3 = FUN_00402720(pbVar6,(int *)&param_3,4,'\x10');
          if (iVar3 == 4) {
LAB_00402631:
            pbVar6 = pbVar5 + 2;
            pbVar2 = param_3;
          }
          else {
            pbVar2 = (byte *)0x0;
          }
          break;
        case (byte *)0x76:
          pbVar2 = (byte *)0xb;
          break;
        case (byte *)0x78:
          pbVar6 = pbVar5 + 2;
          iVar3 = FUN_00402720(pbVar6,(int *)&param_3,2,'\x10');
          if (iVar3 == 2) goto LAB_00402631;
          pbVar2 = (byte *)0x0;
        }
        pbVar6 = pbVar6 + iVar3;
        bVar7 = false;
        if (pbVar2 < (byte *)0x80) {
          if (pbVar2 != (byte *)0x0) {
            *pbVar4 = (byte)pbVar2;
            goto LAB_00402677;
          }
        }
        else {
          iVar3 = FUN_00402780(pbVar4,(ushort)pbVar2);
          pbVar4 = pbVar4 + iVar3;
        }
      }
      else {
        if (!bVar7) {
          *pbVar4 = bVar1;
        }
LAB_00402677:
        pbVar4 = pbVar4 + 1;
      }
      pbVar5 = pbVar6;
      if (param_4 <= pbVar6) {
        return (int)pbVar4 - (int)param_1;
      }
    }
  }
  return (int)pbVar4 - (int)param_1;
}


