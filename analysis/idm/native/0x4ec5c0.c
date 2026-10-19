/* REA ev_9b403c69af76a3931d8bba4eb5243a456abce12e5cc8ec0d1b7aecf20e7ec542; IDMan.exe; decompiler output, not original source. */

void FUN_004ec5c0(void)

{
  int iVar1;
  uint uStack_30;
  undefined4 local_1c;
  undefined4 local_18;
  undefined1 *local_14;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  local_8 = 0xffffffff;
  puStack_c = &LAB_00659d80;
  local_10 = ExceptionList;
  uStack_30 = DAT_007608e0 ^ (uint)&stack0xfffffffc;
  local_14 = (undefined1 *)&uStack_30;
  ExceptionList = &local_10;
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a50,
             "File %s download complete.\n\nDownloaded %s (%I64d Bytes)\nFile saved as %s",0x48);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a54,
             "This feature is available in registered version only.",0x35);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a58,
             "Cannot rename downloaded file from temp folder (%s) to %s. The file has been saved in temp folder."
             ,0x62);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a5c,
             "Password verification failed! Please try again.",0x2f);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a60,"Password field has not been filled!",0x23);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a64,
             "User name field has not been filled! Please try again.",0x36);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a68,
             "Site/path field have not been filled! Please try again",0x36);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a6c,
             "Are you sure you want to delete login information for this site?",0x40);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a70,
             "Your download list has the file that will be downloaded from the ",0x41);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a74,
             ".\nApply these username and password to this download?",0x35);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a78,
             ".\nChange username and password of this download to \"anonymous\" ?",0x40);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a7c,
             "The login information for this site/path already exists in your password list.\nThe new login information hasn\'t been added. You may want to edit the existing one."
             ,0xa2);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a80,
             "You have changed the \"site/path field.\" The login information for this new site/path already exists in your password list.\nThe login information hasn\'t been changed. You may want to edit the existing one."
             ,0xcc);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a84,"This site is temporarily unavailable!",0x25);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a88,
             "Authorization is required for this site/path. You can set login information for this download by selecting properties item in a right click context menu. Or add login information to \"Options/Sites passwords\" to use these login/password every time you are downloading files from this site."
             ,0x120);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a8c,
             "Proxy authorization is required for this proxy server. You can change proxy username and password in \"Options/Proxy\""
             ,0x74);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a98,"Cannot download this file.",0x1a);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766a9c,"Cannot download this file.",0x1a);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766aa0,"Not enough memory to resume download.",0x25);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766aa4,
             "No connection could be made because the target machine actively refused it. \nProbably the service is inactive on the server or you have a firewall installed that blocks Internet Download Manager. If you have a firewall, please ensure that its settings allow IDM downloading from the Internet."
             ,0x124);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766aa8,
             "Permission denied. Please check your firewall settings and ensure that IDM has permits to access the Internet."
             ,0x6e);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766aac,
             "Cannot assign requested address. The requested address is not valid in its context.",
             0x53);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766ab0,
             "An existing connection was forcibly closed by the remote host. This normally results if the peer application on the remote host is suddenly stopped, the host is rebooted, or the remote host used a \'hard close\'"
             ,0xd1);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766ab4,
             "A socket operation failed because the destination host was down. A socket operation encountered a dead host. Networking activity on the local host has not been initiated."
             ,0xaa);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766abc,
             "A socket operation was attempted to an unreachable network. This usually means the local software knows no route to reach the remote host."
             ,0x8a);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766ac0,
             "The host you were connected to crashed and rebooted.",0x34);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766ac4,
             "A socket operation encountered a dead network. This could indicate a serious failure of the network system (i.e. the protocol stack that the WinSock DLL runs over), the network interface, or the local network itself."
             ,0xd8);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766ac8,
             "Too many open network connections. Please wait for some downloads to finish.",0x4c);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766acc,"An error occurred while connecting",0x22);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766ad0,
             "The size of \"%s\" file is %I64d bytes.\r\nThere is no disk space left on drive \"%s\"\' where you want to save this file.\r\nIf you don\'t free some disk space while downloading, the file will be saved in a temporary directory as \r\n%s"
             ,0xe1);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766ad4,
             "There is no disk space left on drive \'%s\' to store downloaded file parts.\nTo continue downloading, free some disk space or change temporary directory location in \"Options->Save to\" and then resume downloading"
             ,0xd0);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766ad8,"Cannot find server %s",0x15);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766adc,"Cannot find proxy server %s",0x1b);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766ae0,"Cannot connect to %s:%ld",0x18);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766ae4,"Cannot connect to proxy server %s:%ld",0x25);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766ae8,
             "Cannot connect to %s:%ld\nConnection timed out",0x2d);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766aec,
             "Cannot connect to proxy server %s:%ld\nConnection timed out",0x3a);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766af0,
             "An unknown error occurred while appending files or joining downloaded parts into one file"
             ,0x59);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766af4,
             "An unknown error occurred while joining downloaded parts into one file",0x46);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766af8,
             "Cannot open local file for writing while joining downloaded parts into one file",0x4f)
  ;
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766afc,
             "Cannot get the size of next part while joining downloaded parts into one file",0x4d);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b00,
             "Virtual memory allocation error while joining downloaded parts into one file",0x4c);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b04,
             "Fatal write error occurred while joining downloaded parts into one file",0x47);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b08,
             "Fatal read error occurred while joining downloaded parts into one file",0x46);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b0c,
             "An unknown file or registry error occurred while rebuilding old downloaded parts",0x50
            );
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b10,"Cannot connect to the server",0x1c);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b14,"Internet socket read error",0x1a);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b18,"Internet socket write error",0x1b);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b1c,"Connection timed out",0x14);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b20,"Connection timed out",0x14);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b24,"Connection timed out, restarting...",0x23);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b28,"Connection failed, restarting...",0x20);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b2c,"An error occurred while receiving data",0x26);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b30,"Connection has been closed by server.",0x25);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b34,"Cannot login to ftp server.",0x1b);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b38,"Cannot find this file on ftp server.",0x24);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b3c,"Ftp server doesn\'t allow connections.",0x25);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b40,
             "Error occurred while receiving ftp server reply.",0x30);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b44,
             "Error occurred while receiving ftp server reply: Invalid reply format",0x45);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b48,
             "Cannot find IP address for data socket in reply on PASV command.",0x40);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b4c,"Cannot open internet socket.",0x1c);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b50,"Cannot open ftp data socket",0x1b);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b54,
             "Cannot bind the address to the internet socket",0x2e);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b58,"Error during call to listen() data socket",
             0x29);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b5c,
             "Error during call to accept() internet socket.",0x2e);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b60,
             "Cannot send PORT command.\nError during call to getsockname().",0x3d);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b64,"PORT command failed.",0x14);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b68,
             "File write error occurred while receiving data from Internet socket.\nThe possible reason: The disk is full."
             ,0x6b);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b6c,
             "Cannot open data connection because ftp server does not support PASV command and checkbox \'Use FTP in PASV mode\' in Options->Proxy/Firewall is turned on."
             ,0x99);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b70,
             "The size of file %s has not been found in server reply",0x36);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b74,
             "The server sent incorrect reply on restart command.",0x33);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b78,
             "Server sent new location and the file name %s has changed to %s.\n\nContinue downloading ?"
             ,0x58);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b7c,"Disconnecting...",0x10);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b80,
             "Download complete, appending all downloaded parts into one file...",0x42);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b84,"Optional connections have been created.",0x27)
  ;
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b88,"Connecting...",0xd);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b8c,"Select failed",0xd);
  ATL::CSimpleStringT<char,0>::SetString((CSimpleStringT<char,0> *)&DAT_00766b90,"Connect OK.",0xb);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b94,
             "Write file error while receiving data from the internet socket.",0x3f);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b98,"Recv() procedure failed",0x17);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766b9c,"Connection has been closed by the server.",
             0x29);
  ATL::CSimpleStringT<char,0>::SetString((CSimpleStringT<char,0> *)&DAT_00766ba0,"Disconnect.",0xb);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766ba4,"Waiting response...",0x13);
  ATL::CSimpleStringT<char,0>::SetString((CSimpleStringT<char,0> *)&DAT_00766ba8,"Reply OK",8);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bac,"Receiving data...",0x11);
  ATL::CSimpleStringT<char,0>::SetString((CSimpleStringT<char,0> *)&DAT_00766bb0,"Completed.",10);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bb4,"Data connection: connecting...",0x1e);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bb8,"Data connection: OK",0x13);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bbc,"Ready to retrieve data",0x16);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bc0,
             "Checking command sequence capability: sending two commands together...",0x46);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bc4,
             "Checking command sequence capability: first reply OK, waiting second reply...",0x4d);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bc8,"Receiving file info...",0x16);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bcc,
             "Cannot find the end of string in server reply.",0x2e);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bd0,
             "Cannot find the end of string in ftp server reply.",0x32);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bd4,"Sending QUIT.",0xd);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bd8,"The server is not ready for new user.",0x25);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bdc,"Control socket connected.",0x19);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766be0,"Sending user name...",0x14);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766be4,"Sending password...",0x13);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766be8,"User name failed",0x10);
  ATL::CSimpleStringT<char,0>::SetString((CSimpleStringT<char,0> *)&DAT_00766bec,"User name OK",0xc)
  ;
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bf0,"Password failed",0xf);
  ATL::CSimpleStringT<char,0>::SetString((CSimpleStringT<char,0> *)&DAT_00766bf4,"Password OK",0xb);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bf8,"Send PASV command...",0x14);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766bfc,"The server does not support PASV.",0x21);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766c00,"PASV OK, extracting address...",0x1e);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766c04,"Bind socket OK, try call listen()",0x21);
  ATL::CSimpleStringT<char,0>::SetString((CSimpleStringT<char,0> *)&DAT_00766c08,"Accepting...",0xc)
  ;
  ATL::CSimpleStringT<char,0>::SetString((CSimpleStringT<char,0> *)&DAT_00766c0c,"Accept OK",9);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766c10,"Sending PORT command...",0x17);
  ATL::CSimpleStringT<char,0>::SetString((CSimpleStringT<char,0> *)&DAT_00766c14,"PORT OK",7);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766c18,"Changing server work directory...",0x21);
  ATL::CSimpleStringT<char,0>::SetString((CSimpleStringT<char,0> *)&DAT_00766c1c,"CWD failed.",0xb);
  ATL::CSimpleStringT<char,0>::SetString((CSimpleStringT<char,0> *)&DAT_00766c20,"CWD OK",6);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766c24,"Connection failed.",0x12);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766c28,"Sending TYPE ...",0x10);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766c2c,"TYPE command failed.",0x14);
  ATL::CSimpleStringT<char,0>::SetString((CSimpleStringT<char,0> *)&DAT_00766c30,"TYPE OK.",8);
  ATL::CSimpleStringT<char,0>::SetString
            ((CSimpleStringT<char,0> *)&DAT_00766c34,"Sending REST command...",0x17);
  ATL::CSimpleStringT<char,0>::SetString((CSimpleStringT<char,0> *)&DAT_00766c38,"REST failed",0xb);
  FUN_00402fd0("REST OK");
  FUN_00402fd0("Sending RETR command...");
  FUN_00402fd0("RETR failed.");
  FUN_00402fd0("Sending SIZE command...");
  FUN_00402fd0("Sending LIST command.");
  FUN_00402fd0("LIST failed.");
  FUN_00402fd0("LIST OK");
  FUN_00402fd0("SIZE failed.");
  FUN_00402fd0("Cannot find file size in reply.");
  FUN_00402fd0("SIZE OK");
  FUN_00402fd0("Control socket is not connected to server.");
  FUN_00402fd0(
              "Restarting at the beginning because local file has been erased. Restarting from arbitrary position is not supported in this case."
              );
  FUN_00402fd0("Connection is restarted...");
  FUN_00402fd0("Probably the server does not support command pipelining.");
  FUN_00402fd0("Command sequence capability: OK");
  FUN_00402fd0("Send HEAD...");
  FUN_00402fd0("Send GET...");
  FUN_00402fd0("Rebuilding old downloaded file parts...");
  FUN_00402fd0("Connecting...");
  FUN_00402fd0("Download complete. Downloaded %I64d Bytes Total");
  FUN_00402fd0("Receiving data...");
  FUN_00402fd0("Optional connections, connecting...");
  FUN_00402fd0("Download failed. Downloaded: %I64d Bytes Total");
  FUN_00402fd0(
              "The size of %s is %I64d bytes.\nThere is no disk space left on drive \'%s\' to store downloaded file parts.\nTo continue downloading free some disk space or remove this file from download list, change temporary directory location in \"Options->Save to\", and then try again."
              );
  FUN_00402fd0("There is no disk space left on drive \'%s\'.");
  FUN_00402fd0("Cannot rebuild downloaded file chunks.");
  FUN_00402fd0("This download is already complete, the local path has not been changed.");
  FUN_00402fd0(
              "The file last modified date has been changed on the server.\nWould you like to download this file from the beginning?"
              );
  FUN_00402fd0(
              "You are using the latest version of Internet Download Manager. Please check back again for updates at a later time."
              );
  FUN_00402fd0("An error occurred while saving entered data, please try again.");
  FUN_00402fd0(
              "An error occurred while allocating virtual memory. Try to close some applications and then press \'Retry\'."
              );
  FUN_00402fd0("Rebuilding");
  FUN_00402fd0(
              "The requested server name is valid, but it does not have the correct associated data being resolved for (The host itself exists, but is not directly reachable)."
              );
  FUN_00402fd0(
              "File %s - downloaded %s (%I64d Bytes). The file may not have been downloaded completely, because the file size is unknown.\nFile saved as %s"
              );
  FUN_00402fd0("No items selected!");
  FUN_00402fd0("Move up");
  FUN_00402fd0("Move down");
  FUN_00402fd0("Delete item");
  FUN_00402fd0("Cannot download this file. Invalid http server reply.");
  FUN_00402fd0("QuickUpdate for IDM from www.internetdownloadmanager.com. Receiving new files...");
  FUN_00402fd0("Internet Download Manager Manual");
  FUN_00402fd0("Download has been cancelled by user");
  FUN_00402fd0(
              "Quick update failed, an error occurred while downloading new IDM files. Try to update later."
              );
  FUN_00402fd0("The requested file doesn\'t exist.");
  FUN_00402fd0("\nWould you like to open it?");
  FUN_00402fd0("The server replies that you don\'t have permissions to download this file.");
  FUN_00402fd0("An error occurred while reading the phone book.");
  FUN_00402fd0("Cannot connect to the Internet.");
  FUN_00402fd0("Cannot download the file because of timeout");
  local_8 = 0;
  FUN_0052cd60(&DAT_0076756c,&DAT_00767560,&DAT_00767564,&DAT_00767568);
  local_8 = 0xffffffff;
  FUN_00402fd0(
              "The requested file looks like html web page.\r\n\r\nIf this download was taken over from your browser automatically when you clicked on a link, try to hold Alt key when clicking on the link to let IE open the page.\r\n\r\nWould you like to continue downloading the file with IDM and save it as html page?"
              );
  FUN_00402fd0(
              "The downloaded file looks like html web page.\r\n\r\nIf this download was taken over from your browser automatically when you clicked on a link, try to hold Alt key when clicking on the link to let IE open the page.\r\n\r\nWould you like to rename the downloaded file to html page?"
              );
  local_8 = 2;
  FUN_0052cba0(&DAT_0076755c);
  local_8 = 0xffffffff;
  FUN_00402fd0(
              "Could not change integration of Internet Download Manager into Internet Explorer because you might have no access rights to the system registry"
              );
  FUN_00402fd0("The file has been moved.");
  FUN_00402fd0("The %s file has been moved.");
  FUN_00402fd0("Opening downloaded file");
  FUN_00402fd0("Download IDM component");
  FUN_00402fd0(
              "The file\r\n%s\r\nwill be saved as\r\n%s\r\nSuch file already exists on your computer. Do you want to replace existing file?\n\n Press \"YES\" to replace, \"NO\" to save the file with different name, \"CANCEL\" to cancel adding the file to your download list."
              );
  FUN_00402fd0(
              "The file\r\n%s\r\nwill be saved as\r\n%s\r\nSuch file already exists in your download list. Do you confirm saving the file with this name?\n\n Press \"YES\" to confirm, \"NO\" to save the file with a different name, \"CANCEL\" to cancel adding the file to your download list."
              );
  FUN_00402fd0("Import to IDM");
  FUN_00402fd0("Cannot read links from the file or the file is corrupted");
  FUN_00402fd0("The file is not a valid IDM export file or the file is corrupted");
  FUN_00402fd0("The file doesn\'t have any web links");
  FUN_00402fd0("An unknown error occurred while executing the operation!");
  FUN_00402fd0(
              "Cannot download this file, maybe this ftp server doesn\'t support download resume. Would you like to add the same file and download it from the beginning?"
              );
  FUN_00402fd0(
              "The name to save the file was \"%s%s\", but the server sent the following name \"%s\". Would you like to save the file with the file name that was received from the server?"
              );
  FUN_00402fd0(
              "File %s download complete.\nThis m3u file contains a web link to one mp3 file. Would you like to download this mp3 file?"
              );
  FUN_00402fd0(
              "File %s download complete.\nThis m3u file contains one or more web links to mp3 files. Would you like to add these mp3 files to your download list?"
              );
  FUN_00402fd0("M3U file downloaded.");
  FUN_00402fd0("Cannot start virus checking program.\n%s");
  FUN_00402fd0(
              "Internet Download Manager has finished queue processing. According to scheduler settings IDM is going to hang up your modem."
              );
  FUN_00402fd0("Hang up modem");
  FUN_00402fd0("Hang up now");
  FUN_00402fd0("Stop now");
  FUN_00402fd0("Download limits exceeded!");
  FUN_00402fd0("Invalid data");
  FUN_00402fd0(
              "The value of hours in \"Download limits\" must be between 1 and 23. Please correct or uncheck the \"Download limits\" box."
              );
  FUN_00402fd0("Would you like to download tutorial (about 60 KB) from IDM home site?");
  FUN_00402fd0(
              "Would you like to download and install IDM help file (about 200 KB) from IDM home site ?"
              );
  FUN_00402fd0("Cannot find HHCtrl.ocx to display help file!");
  FUN_00402fd0("IDM help");
  FUN_00402fd0(
              "The file size has been changed on the server.\nIt will be downloaded from the beginning."
              );
  FUN_00402fd0("Cannot create connection via proxy server.");
  FUN_00402fd0("An error occurred while creating security connection to %s.");
  FUN_00402fd0("An error occurred while encrypting request message.");
  FUN_00402fd0("An error occurred while loading security dll. Cannot create https connectin.");
  FUN_00402fd0("Internet Download Manager options");
  FUN_00402fd0("Download complete");
  FUN_00402fd0("Download failed");
  FUN_00402fd0("Queue processing stopped/finished");
  FUN_00402fd0("Queue processing started");
  FUN_00402fd0("Event");
  FUN_00402fd0("Sound file");
  FUN_00402fd0("Cannot copy selected sound file to IDM sounds folder!");
  FUN_00402fd0(
              "You have %d days left to use Internet Download Manager. Do you want to register your copy of IDM now?"
              );
  FUN_00402fd0("File Name");
  FUN_00402fd0("Packed");
  FUN_00402fd0(&DAT_0068ff3c);
  FUN_00402fd0("Modified");
  FUN_00402fd0("All Downloads");
  FUN_00402fd0("Unfinished");
  FUN_00402fd0("Finished");
  FUN_00402fd0("Programs");
  FUN_00402fd0("Music");
  FUN_00402fd0("Video");
  FUN_00402fd0("Games");
  FUN_00402fd0("Documents");
  FUN_00402fd0("Add category");
  FUN_00402fd0("Delete");
  FUN_00402fd0("Properties");
  FUN_00402fd0("Do you really want to delete \"%s\" category from IDM categories list?");
  FUN_00402fd0("Deleting a category from IDM categories list");
  FUN_00402fd0("Adding a category to IDM categories list");
  FUN_00402fd0("Editing category properties");
  FUN_00402fd0("Please fill in the \"Category name\" field.");
  FUN_00402fd0("The folder \"%s\" doesn\'t exist.\r\nDo you want to create the folder?");
  FUN_00402fd0("Cannot create the folder!\r\n%s");
  FUN_00402fd0("Hide categories");
  FUN_00402fd0("Show categories");
  FUN_00402fd0("Choose a directory");
  FUN_00402fd0("General");
  FUN_00402fd0(
              "Would you like to place all old downloads to these categories according to files types? The files won\'t be moved on the hard drive, but will be associated with their designated download categories."
              );
  FUN_00402fd0(
              "Would you like to place all old downloads with \"%s\" file types to this category? The files won\'t be moved on the hard drive, but will be associated with this category."
              );
  FUN_00402fd0("IDM categories");
  FUN_00402fd0(
              "IDM got \"Access denied\" error while assembling downloaded parts of the file. It might happen because some application like anti virus program or firewall opened the file, but did not close it correctly, or you tried to open the file while it was downloading. You should reboot your computer and then click on resume button to assemble the file."
              );
  FUN_00402fd0("Default download directory for \"%s\" category");
  FUN_00402fd0("Change folder for \"%s\" category on last selected");
  FUN_00402fd0("General");
  FUN_00402fd0("Connection");
  FUN_00402fd0("Downloads");
  FUN_00402fd0("Sites Logins");
  FUN_00402fd0("Dial Up");
  FUN_00402fd0("Sounds");
  FUN_00402fd0("General");
  FUN_00402fd0("Connection");
  FUN_00402fd0("Downloads");
  FUN_00402fd0("Proxy");
  FUN_00402fd0("Logins");
  FUN_00402fd0("DialUp");
  FUN_00402fd0("Sounds");
  FUN_00402fd0("Browser/System Integration");
  FUN_00402fd0("Connections and Limits");
  FUN_00402fd0("Default download settings");
  FUN_00402fd0("User names and passwords for servers/sites");
  FUN_00402fd0("Dial up settings");
  FUN_00402fd0("Sound settings");
  FUN_00402fd0("- Select your internet connection -");
  FUN_00402fd0("About IDManager");
  FUN_00402fd0("Restore");
  FUN_00402fd0(&DAT_006a18b0);
  FUN_00402fd0("Contact us");
  FUN_00402fd0(&DAT_006a189c);
  FUN_00402fd0("Order online");
  FUN_00402fd0("Registration");
  FUN_00402fd0("Stop Download");
  FUN_00402fd0("Remove");
  FUN_00402fd0("Resume Download");
  FUN_00402fd0("Properties");
  FUN_00402fd0("Invalid URL entered. Please correct.");
  FUN_00402fd0("This download is not complete. Proceed with deleting?");
  FUN_00402fd0("Could not find proxy configuration in System Settings");
  FUN_00402fd0(
              "The file with this URL has been already downloaded by Internet Download Manager. Do you want to download it again?"
              );
  FUN_00402fd0(
              "The file with this URL already exists in your list and waits for download to complete. Do you want to resume file?"
              );
  FUN_00402fd0("Too many files with this name. Please delete them or change local names");
  FUN_00402fd0("Please enter login name.");
  FUN_00402fd0("Could not find associated Browser");
  FUN_00402fd0("Unable to find Quick Update application.");
  FUN_00402fd0("Please enter registration name.");
  FUN_00402fd0("Invalid registration number entered.");
  FUN_00402fd0("Download link was found in clipboard. Do you want to download it?");
  FUN_00402fd0("Please enter your first name");
  FUN_00402fd0("Please enter your last name");
  FUN_00402fd0("Please enter your email");
  FUN_00402fd0("Please enter your Serial Number");
  FUN_00402fd0(
              "You have entered incorrect Serial Number.\r\n\r\nPlease don\'t mix 0(zero) and O(ou), I and 1 while typing your S/N! CUT and PASTE your S/N!"
              );
  FUN_00402fd0(
              "Internet Download Manager has been registered with a counterfeit Serial Number. IDM is exiting..."
              );
  FUN_00402fd0(
              "Internet Download Manager has been registered with a counterfeit Serial Number or the Serial Number has been blocked. IDM is exiting..."
              );
  FUN_00402fd0(
              "Internet Download Manager has not been registered for 30 days. Trial period is over and IDM is exiting..."
              );
  FUN_00402fd0("Internet Download Manager has not been registered for 15 days. IDM is exiting...");
  FUN_00402fd0("Unknown error during");
  FUN_00402fd0("Unknown");
  FUN_00402fd0("Add to download queue");
  FUN_00402fd0("Delete from queue");
  FUN_00402fd0("Start queue");
  FUN_00402fd0("Stop queue");
  FUN_00402fd0(&DAT_006a126c);
  FUN_00402fd0("Last try date:  ");
  FUN_00402fd0("Result:\n");
  FUN_00402fd0("Select only one file to open!");
  FUN_00402fd0(&DAT_006a126c);
  FUN_00402fd0(
              " already exists on your computer. Do you want to replace existing file?\n Press \"YES\" to replace, \"NO\" to save the file with different name, \"CANCEL\" to cancel download."
              );
  FUN_00402fd0("Download from");
  FUN_00402fd0("Save to");
  FUN_00402fd0("Do you really want to delete all completed downloads?");
  FUN_00402fd0(&DAT_006a1128);
  FUN_00402fd0("File Name");
  FUN_00402fd0("Status");
  FUN_00402fd0(&DAT_0068ff3c);
  FUN_00402fd0("Time left");
  FUN_00402fd0("Transfer rate");
  FUN_00402fd0("Description");
  FUN_00402fd0("Add URL");
  FUN_00402fd0("Resume");
  FUN_00402fd0(&DAT_006a10f4);
  FUN_00402fd0("Stop All");
  FUN_00402fd0("Delete");
  FUN_00402fd0("Delete Co...");
  FUN_00402fd0("Options");
  FUN_00402fd0("Scheduler");
  FUN_00402fd0("Tell a Friend");
  FUN_00402fd0("Start Queue");
  FUN_00402fd0("Stop Queue");
  FUN_00402fd0("Show drop target");
  FUN_00402fd0(
              "Warning: This computer program is protected by copyright laws and international treaties. Unauthorized reproduction or distribution of this program, or any portion of it, may result in severe civil and criminal penalties, and will be prosecuted to the maximum extent possible under law."
              );
  FUN_00402fd0("Cannot create folder %s, error code = %ld\n");
  FUN_00402fd0("Error");
  FUN_00402fd0("Files");
  FUN_00402fd0("All Files");
  FUN_00402fd0("Details:\n");
  FUN_00402fd0("Completed.");
  FUN_00402fd0("Pause");
  FUN_00402fd0("Download complete. Virus checking in progress...");
  FUN_00402fd0("AVI: Res. %dx%d, %d samp per sec, Length %s");
  FUN_00402fd0(", Length: ");
  FUN_00402fd0(", Title: ");
  FUN_00402fd0(", Artist: ");
  FUN_00402fd0(", Album: ");
  FUN_00402fd0(", Genre: ");
  FUN_00402fd0("WAV: %d samp per sec, %d bits per samp, Length: %s");
  FUN_00402fd0("MPEG: Res. %dx%d, %.1f samp per sec, %d bits per samp");
  FUN_00402fd0("%s,   total %ld files");
  FUN_00402fd0(" Trial");
  FUN_00402fd0(" Full");
  FUN_00402fd0("Redial  %ld");
  FUN_00402fd0("Undefined RAS Dial Error.");
  FUN_00402fd0(
              "The language changes on the main dialog will apply only after restarting IDM. Do you want to restart IDM now?"
              );
  FUN_00402fd0(
              "From %d:%02d to %d:%02d you downloaded %ld MB. All IDM downloads will be stopped because you have exceeded your download limits set in in IDM Scheduler (or \'Options->Connections\')!"
              );
  FUN_00402fd0(
              "From %d:%02d to %d:%02d you downloaded %ld MB. All IDM downloads have been stopped at %d:%02d because you had exceeded your download limits set in IDM Scheduler (or \'Options->Connections\')!\r\n\r\nAll stopped downloads will be resumed automatically at %02d:%02d. If you want to resume downloads immediately, change settings in Download Limits and press on Resume."
              );
  FUN_00402fd0("Complete");
  FUN_00402fd0("Rebuilding...");
  FUN_00402fd0("Disconnecting...");
  FUN_00402fd0("Text Files");
  FUN_00402fd0("IDM Export Files");
  FUN_00402fd0("Save as...");
  FUN_00402fd0(&DAT_006a0ae8);
  FUN_00402fd0("Downloaded");
  FUN_00402fd0(&DAT_006a0ad4);
  FUN_00402fd0(&DAT_006a0ad0);
  FUN_00402fd0(&DAT_006a0acc);
  FUN_00402fd0("%ld hour(s) %d min");
  FUN_00402fd0("%d min %d sec");
  FUN_00402fd0("%d sec");
  FUN_00402fd0("Start");
  FUN_00402fd0("Show details >>");
  FUN_00402fd0("<< Hide details");
  FUN_00402fd0(
              "Please don\'t close download dialog when IDM is assembling file, otherwise data may be damaged."
              );
  FUN_00402fd0("Quick Update");
  FUN_00402fd0("Warning");
  FUN_00402fd0("Choose a directory");
  FUN_00402fd0("Virus scanner program");
  FUN_00402fd0("Site/path");
  FUN_00402fd0(&DAT_00691bf4);
  FUN_00402fd0("Password");
  FUN_00402fd0("Sound Files");
  FUN_00402fd0("complete");
  FUN_00402fd0("File Properties");
  FUN_00402fd0("Please use English");
  FUN_00402fd0("Unknown");
  FUN_00402fd0("Show normally sized");
  FUN_00402fd0("Show minimized");
  FUN_00402fd0("Don\'t show");
  FUN_00402fd0("Please purchase and install the full-featured version of IDM at first!");
  FUN_00402fd0("Customize...");
  FUN_00402fd0("Date Added");
  FUN_00402fd0("Last Try Date");
  FUN_00402fd0("Save To");
  FUN_00402fd0("Referer");
  FUN_00402fd0("Columns...");
  FUN_00402fd0("Compressed");
  FUN_00402fd0("Server");
  FUN_00402fd0("Number");
  FUN_00402fd0("File Type");
  FUN_00402fd0("Link Text");
  FUN_00402fd0(
              "The name to save the file was \"%s\", but the server sent the following file type \"%s\", and the file should be saved as  \"%s\". Would you like to save the file with the name matched the file type received from the server?"
              );
  FUN_00402fd0("Redownload");
  FUN_00402fd0("Open folder");
  FUN_00402fd0(
              "\"%s\" data transmission protocol is not supported by IDM at this time. Or it might be a spelling error, therefore please check the spelling, and try again."
              );
  FUN_00402fd0(
              "IDM has received \"Access Denied\" error while assembling file parts. It might happen because the file was opened by another application like antivirus software, or you tried to open it yourself. In order to assemble this file, you will need to reboot your computer, select the file in IDM, and press on resume button."
              );
  FUN_00402fd0(
              "Please register IDM before updating.\r\n Pick \"Registration->Registration\" menu item and enter your registration information. Then pick \"Help->Quick Update\" menu item to start updating."
              );
  FUN_00402fd0(
              "According to our license agreement that you have accepted, you are allowed to update Internet Download Manager freely during one year after the purchase. If you want to continue updating IDM after this term, you will need to subscribe to update service.\r\n Would you like to look at IDM subscription page?"
              );
  FUN_00402fd0(
              "The key(s) to prevent downloading should not be contained in the key(s) to force downloading. \r\n Please choose others keys."
              );
  FUN_00402fd0(
              "IDM cannot find %d file(s) that are necessary for browser and system integration. Would you like to download them?"
              );
  FUN_00402fd0(
              "Error connecting. No such host is known. The name is not an official hostname or alias, or it cannot be found in the database(s) being queried.\r\n\r\nFind below possible reasons for this error:\r\n\r\n1. You have a firewall application on your computer, and you have just installed, replaced or updated your current version of Internet Download Manager. Please check your firewall settings and ensure that IDM has permits to access the Internet.\r\n\r\n2. A service that finds servers in the Internet called Domain Name Service (DNS) has temporary problems, or the network connection between DNS and your computer gets broken. It\'s possible that you need to reboot your computer.\r\n\r\n3. There is a spelling mistake in the server name. Please check your spelling.\r\n"
              );
  FUN_00402fd0("Internet Download Manager Network Integration");
  FUN_00402fd0(
              "Internet Download Manager detected that its network integration had been deleted by some 3rd party application. Would you like to restore the network integration of Internet Download Manager?"
              );
  FUN_00402fd0(
              "Internet Download Manager detected that some 3rd party application had changed IDM network integration. This may happen because of a newly installed network application, or because of an application that restores registry records erroneously. Would you like to restore the network integration of Internet Download Manager?"
              );
  FUN_00402fd0(
              "Internet Download Manager network integration has been changed. We recommend you to reboot your computer. Would you like to reboot now?"
              );
  FUN_00402fd0(
              "Internet Download Manager network integration has been changed. You need to reboot your computer, because your firewall is not able to detect these changes, and Internet Applications will not communicate in the Internet properly until reboot.\r\n\r\nPlease save your work and close all applications now, the computer will reboot after you press OK button."
              );
  FUN_00402fd0(
              "If you want to integrate Internet Download Manager into an application that is running, you will need to restart the application, because IDM cannot integrate into applications that are already running.\r\n\r\nAfter integrating IDM into the application you may turn off \"Detect New Applications\" option for better performance."
              );
  FUN_00402fd0("Detect New Applications");
  FUN_00402fd0(
              "IDM has detected that you are using a proxy server in Internet Explorer. Would you like to use the same proxy settings in IDM?"
              );
  FUN_00402fd0("First run of Internet Download Manager");
  FUN_00402fd0(
              "IDM has detected that you are using Windows DialUp networking to connect your modem to the Internet. Would you like to use these settings in IDM?"
              );
  FUN_00402fd0(
              "IDM is downloading a small test page to check your Internet connection. Please wait a few seconds"
              );
  FUN_00402fd0(
              "IDM is testing your Internet Explorer. Please wait a few seconds while Internet Explorer loads a small test page"
              );
  FUN_00402fd0(
              "Cannot download the test page. It\'s possible that Internet Download Manager network integration conflicts with another Internet application or a firewall. IDM will turn off advanced browser integration to restore your original settings. You may turn on advanced browser integration later in IDM->Options->General dialog"
              );
  FUN_00402fd0(
              "You need to reboot your computer in order to restore original settings. Please save your work and close all applications now, the computer will reboot after you press OK button."
              );
  FUN_00402fd0(
              " Internet Download Manager network integration has been installed successfully. In a rare case the network integration may conflict with a new installed 3rd-party Internet application. If you find any problems with network integration, please uncheck \"Use advanced browser integration\" box in IDM->Options->General dialog"
              );
  FUN_00402fd0(
              "You have turned off advanced browser integration. If you found a problem with advanced browser integration, we would like to fix the problem for you. In order to fix the problem, we need to collect and analyze some technical information about your computer configuration and installed Internet applications. IDM will collect and show you this information on the next stage before sending it to our support department.\r\n\r\nWould you like IDM to generate this information?"
              );
  FUN_00402fd0("Turning off advanced browser integration");
  FUN_00402fd0(
              "IDM is trying to download a test page to check your Internet connection. Please ensure that you are connected to the Internet. If you have a firewall, please ensure that the new version of IDM has permissions to access the Internet before proceeding. Then press \"Check\" button below, and IDM will try to download the test page again."
              );
  FUN_00402fd0(
              "IDM has changed its network integration and need to check your Internet connection again. If you are not connected to the Internet please connect and press \"Check\" button."
              );
  FUN_00402fd0("Sending data...\r\nPlease wait a few seconds");
  FUN_00402fd0(
              "We have created a configuration report that you can send us. Please send us the following information to help us to fix this problem in future versions of IDM. This data is of technical kind. We don\'t collect your personal information, and we will treat this report as confidential and anonymous."
              );
  FUN_00402fd0(
              "The switching between basic and advanced browser integrations may require system reboot. Are you sure you want to continue?"
              );
  FUN_00402fd0(
              "Internet Download Manager network integration has been changed. You need to reboot your computer, because your Internet Applications will not communicate in the Internet properly until reboot.\r\n\r\nPlease save your work and close all applications now, the computer will reboot after you press OK button."
              );
  FUN_00402fd0(
              "This version of Internet Download Manager has Advanced Browser Integration feature. The installation of this feature may require SYSTEM REBOOT.\r\n\r\nWhen Advanced Browser Integration is turned on, IDM can catch those downloads which were impossible to catch before in IDM, or in any other download manager. Also when using the integration, IDM can be integrated into any browsers and Internet applications to take over your FTP/HTTP downloads.\r\n\r\nIn a rare case this integration may conflict with a new installed 3rd-party Internet application. If you find any problems with network integration, you may uncheck \"Use advanced browser integration\" box in IDM->Options->General dialog to turn it off.\r\n\r\nDo you want to install Advanced Browser Integration?"
              );
  FUN_00402fd0("Advanced Browser Integration has been installed successfully!");
  FUN_00402fd0(
              "Internet Download Manager automatically captured the download link from another Internet Application when it tried to download a file. If you DON\'T WANT to download this file with IDM, please open Options dialog by clicking on Options button\r\n\r\n1. If you don\'t want to download this type of files with IDM, you may delete the corresponding file type (file extension) from the list of automatically captured file types.\r\n\r\n2. If you don\'t want to download any files from a particular site with IDM, you may add this site to the exclude list of sites."
              );
  FUN_00402fd0("All Files");
  FUN_00402fd0("Images Files");
  FUN_00402fd0("Video Files");
  FUN_00402fd0("Audio Files");
  FUN_00402fd0("Executable Files");
  FUN_00402fd0("Compressed Files");
  FUN_00402fd0("PDF Document Files");
  FUN_00402fd0("Don\'t use exclude filter");
  FUN_00402fd0("Site Map");
  FUN_00402fd0("Settings");
  FUN_00402fd0("Filter name");
  FUN_00402fd0(&DAT_0069e7fc);
  FUN_00402fd0("Please fill the filter name");
  FUN_00402fd0("Please fill the mask");
  FUN_00402fd0("The filter with \"%s\" name already exists. Please choose another name.");
  FUN_00402fd0(
              "This error message is possible if IDM was used on several computers under the same Serial Number. Please note that IDM has per computer license. In order to use IDM on several computers at the same time please obtain licenses for each one."
              );
  FUN_00402fd0("Small Buttons");
  FUN_00402fd0("Large Buttons");
  FUN_00402fd0(&DAT_006856d0);
  FUN_00402fd0("Look for new toolbars");
  FUN_00402fd0("Grabber");
  FUN_00402fd0(&DAT_0069e64c);
  FUN_00402fd0(&DAT_0069e648);
  FUN_00402fd0(&DAT_0069e644);
  FUN_00402fd0(&DAT_0069e640);
  FUN_00402fd0("Bytes");
  FUN_00402fd0("Custom settings");
  FUN_00402fd0("All Pictures of the web site");
  FUN_00402fd0("Audio files from the start page and from one next level");
  FUN_00402fd0("All Video files of the web site");
  FUN_00402fd0("All files of the web site except web pages and images");
  FUN_00402fd0("The whole web site");
  FUN_00402fd0(
              "Find below possible reasons for this error:\r\n\r\n1. The Internet Connection not found. Please check your Internet Connection.\r\n\r\n2. You have a firewall application on your computer, and you have just installed, replaced or updated your current version of Internet Download Manager. Please check your firewall settings and ensure that IDM has permits to access the Internet.\r\n\r\n3. A service that finds servers in the Internet called Domain Name Service (DNS) has temporary problems, or the network connection between DNS and your computer gets broken. It\'s possible that you need to reboot your computer.\r\n"
              );
  FUN_00402fd0(
              "When trying to resume the download, Internet Download Manager got a response from the server that it doesn\'t support resuming the download. \r\nIt\'s possible that it\'s a temporary error of the server, and you will be able to resume the file at a later time, but at this time IDM can download this file from the beginning. \r\n\r\n Do you want to download this file from the beginning?"
              );
  FUN_00402fd0(
              "Cannot rebuild the downloaded file. The build sequence of downloaded parts got broken. Find below possible causes of this problem.\r\n\r\n 1.\tThe computer has been shut down unexpectedly because of power outages during downloading/assembling file parts.\r\n 2.\tAnother application or user killed Internet Download Manager process during downloading/assembling file parts. Please notice that it may take a while to rebuild large files. Please don\'t worry, be patient, and wait until IDM finishes assembling. Please don\'t terminate Internet Download Manager process from Task Manager.\r\n\r\nIf these events did not occur, please send %s file (if it exists) to Internet Download Manager support department for analysis."
              );
  FUN_00402fd0("Web content");
  FUN_00402fd0("Advanced");
  FUN_00402fd0("Grabber projects");
  FUN_00402fd0("IDM Grabber");
  FUN_00402fd0("Please enter start page/address");
  FUN_00402fd0("Start page/address");
  FUN_00402fd0("Please enter login and password or uncheck the \"Use Authorization\" checkbox");
  FUN_00402fd0("Please enter manual login page or uncheck the corresponding checkbox");
  FUN_00402fd0("Manual login page");
  FUN_00402fd0("No checked files!");
  FUN_00402fd0("%ld files were added to the main IDM list.");
  FUN_00402fd0(
              "Cannot add files. Too many files are checked. Please download them in this project by clicking \"Download Checked\" toolbar button."
              );
  FUN_00402fd0(
              "Cannot add new files. The main IDM list contains too many files and it may slow down IDM. Please delete some files from the main list."
              );
  FUN_00402fd0("This name is reserved. Please select another name for your template.");
  FUN_00402fd0("The template with such name already exists. Do you want to replace the template?");
  FUN_00402fd0("Save the grabber project?");
  FUN_00402fd0("Open project");
  FUN_00402fd0("Schedule project");
  FUN_00402fd0("Delete project");
  FUN_00402fd0("Please close the running grabber project at first.");
  FUN_00402fd0("Schedule the grabber project");
  FUN_00402fd0("Coming soon!!!\r\n\r\nCheck this feature in the next version of IDM");
  FUN_00402fd0("Exploring site");
  FUN_00402fd0("Downloading");
  FUN_00402fd0("Complete");
  FUN_00402fd0("Stopped");
  FUN_00402fd0("The project with \"%s\" name already exists. Please select another name.");
  FUN_00402fd0("There are no saved projects of IDM grabber");
  FUN_00402fd0("Please select the project name!");
  FUN_00402fd0("Do you want to delete the \"%s\" grabber project?");
  FUN_00402fd0("Invalid number of files explored at the same time. Please set a correct number.");
  FUN_00402fd0("Invalid number of files downloaded at the same time. Please set a correct number.");
  FUN_00402fd0("Cannot open the file \"%s\"\r\n\r\n%s");
  FUN_00402fd0(
              "Would you like to download and install IDM Grabber help file (about 100 KB) from IDM home site ?"
              );
  FUN_00402fd0("Cannot load the project.\r\nMemory allocation error.");
  FUN_00402fd0(
              "Cannot load the project.\r\nThe file of grabber project is corrupt or its version is not acceptable."
              );
  FUN_00402fd0("The file of grabber project is corrupt. Several files have not been loaded");
  FUN_00402fd0(
              "Cannot load the template.\r\nThe file of template information is corrupt or its version is not acceptable."
              );
  FUN_00402fd0("Loading grabber template");
  FUN_00402fd0("Saving grabber template");
  FUN_00402fd0("Saving grabber project");
  FUN_00402fd0("Loading grabber project");
  FUN_00402fd0("IDM Site Grabber. Step %d --- %s");
  FUN_00402fd0("Set start page");
  FUN_00402fd0("Save files to");
  FUN_00402fd0("Set explorer filters");
  FUN_00402fd0("Set file filters");
  FUN_00402fd0("Manual login page");
  FUN_00402fd0("Press the OK button when login process completes");
  FUN_00402fd0(&DAT_0069d654);
  FUN_00402fd0("Cancel");
  FUN_00402fd0(
              "Note: If you plan to take over downloads from Internet Explorer only, then you don\'t need this feature, and you should answer \"No\"."
              );
  FUN_00402500();
  local_8 = 4;
  iVar1 = FUN_0052d950(&local_1c);
  if (iVar1 != 0) {
    FUN_00402f70(&DAT_007672a8,&DAT_0067def0,local_1c);
  }
  FUN_00402500();
  local_8 = CONCAT31(local_8._1_3_,5);
  iVar1 = FUN_0052d030(&local_18);
  if (iVar1 != 0) {
    DAT_00765c34 = 1;
    FUN_00402f70(&DAT_007672ac,&DAT_0067def0,local_18);
  }
  if (DAT_00765c0c != 0) {
    DAT_00765c34 = 0;
  }
  FUN_00402fd0("Internet Download Manager Configuration");
  FUN_00402fd0("Connecting to");
  FUN_00402fd0("Realm:");
  FUN_00402fd0("Check all selected files");
  FUN_00402fd0("Uncheck all selected files");
  FUN_00402fd0("There is no asterisk wildcard in URL to create a download file list!");
  FUN_00402fd0("Batch download");
  FUN_00402fd0(&DAT_0069d490);
  FUN_00402fd0("Save to directory/category");
  FUN_00402fd0("Please enter the logout page or uncheck the corresponding checkbox");
  FUN_00402fd0("Please wait...");
  FUN_00402fd0("Deleting temporary files");
  FUN_00402fd0("Saving Grabber project");
  FUN_00402fd0("Deleting Grabber project");
  FUN_00402fd0("Applying changes");
  FUN_00402fd0("Loading Grabber project");
  FUN_00402fd0(
              "There is not enough free space on drive %s to process the grabber project. Please free some space and try again."
              );
  FUN_00402fd0(
              "There is not enough free space on drive %s to open the grabber project. Please free some space and try again."
              );
  FUN_00402fd0("Cannot open the project. Some of project files have been corrupted or deleted.");
  FUN_00402fd0("Complete start page with all pictures, styles, and scripts for offline browsing");
  FUN_00402fd0("Complete start page");
  FUN_00402fd0("Checking for changes");
  FUN_00402fd0("Downloading the manual login page");
  FUN_00402fd0("Cannot start downloading the manual login page. Please check the URL syntax.");
  FUN_00402fd0("All Files");
  FUN_00402fd0("Link View");
  FUN_00402fd0("Folder View");
  FUN_00402fd0("Other sites");
  FUN_00402fd0("sites");
  FUN_00402fd0("Exclude filters");
  FUN_00402fd0("Include filters");
  FUN_00402fd0("Thumbnails View");
  FUN_00402fd0("No downloaded files");
  FUN_00402fd0("Not Found");
  FUN_00402fd0("Forbidden");
  FUN_00402fd0("Error");
  FUN_00402fd0(
              "To use project scheduler you should save the project at first. Save the project now?"
              );
  FUN_00402fd0(
              "Internet Download Manager has finished processing of grabber project. According to project scheduler settings your computer is shutting down."
              );
  FUN_00402fd0(
              "The scheduled grabber project could not be started at %d:%02d because the project was running at this time."
              );
  FUN_00402fd0(
              "The scheduled grabber project could not be started at %d:%02d because the project settings were being edited at this time."
              );
  FUN_00402fd0(
              "The Java-script will not be processed for the web pages which have been already explored/processed. If you want to process Java-script for these pages you will need to press \"Update all\" toolbar button on the Grabber Action dialog."
              );
  FUN_00402fd0(
              "The update button will update the project by (a) starting to explore the site from the start page, (b) updating files if they have been changed on the server, and (c) downloading new files."
              );
  FUN_00402fd0(
              "The update button will update the project by (a) starting to explore the site from the start page, (b) updating downloaded and checked files if they have been changed on the server."
              );
  FUN_00402fd0(&DAT_0069cc08);
  FUN_00402fd0("Rename project");
  FUN_00402fd0("Minimize to system tray");
  FUN_00402fd0("Restore all download windows");
  FUN_00402fd0("You may turn on this dialog in \"Options->Downloads\"");
  FUN_00402fd0("IDM settings");
  FUN_00402fd0("Downloaded %s (%I64d Bytes)");
  FUN_00402fd0("The file may not have been downloaded completely because the file size is unknown.")
  ;
  FUN_00402fd0("Remember this path for \"%s\" category");
  FUN_00402fd0("Download complete");
  FUN_00402fd0("IDM Downloads");
  FUN_00402fd0(
              "IDM checks for a new release every 2 weeks by requesting an update page from its home site. If your firewall is blocking the update request of IDM and shows a corresponding warning message, you should understand that the request is OK and it\'s safe to allow it."
              );
  FUN_00402fd0("No minimized downloads");
  FUN_00402fd0("Browse");
  FUN_00402fd0("The file types that are not listed in any other category");
  FUN_00402fd0(
              "You pressed on \"Download Later\" button, but actually IDM had downloaded a part of the file already. IDM always starts downloading while displaying \"Download File Info\" dialog. You may turn this option off in \"Options->Downloads\" dialog"
              );
  FUN_00402fd0("Download later");
  FUN_00402fd0(
              "Temporary directory is required for storing file parts during download.\nIf you have several physical drives on your computer, you should select different physical drives for temporary directory and \"Save To\" folders for faster assembling of downloaded files."
              );
  FUN_00402fd0("Automatically put in \"%s\" category the following file types:");
  FUN_00402fd0("Save to");
  FUN_00402fd0("Categories, file types, folders");
  FUN_00402fd0("Rebooting in %d seconds");
  FUN_00402fd0("Reboot now");
  FUN_00402fd0("File types");
  FUN_00402fd0("Downloaded file types");
  FUN_00402fd0("Close");
  FUN_00402fd0(
              "Note: Opera and OLD Mozilla based browsers can be integrated with IDM using plugins. This integration type does not support special keys to prevent or force downloading with IDM from these browsers. If you want to prevent or force downloading with IDM by pressing special keys, you should use \"Advanced browser integration\"."
              );
  FUN_00402fd0(
              "Note: Opera and OLD Mozilla based browsers can be integrated with IDM using plugins. This integration type does not support server exceptions list to prevent downloading with IDM. Also when you change file types list you need to restart these browsers to reload plugins.\nThese restrictions are not applied if you use \"Advanced browser integration\" on \"General\" tab."
              );
  FUN_00402fd0("IDM browser integration");
  FUN_00402fd0(
              "IDM cannot find %s browser on your computer. Please locate the browser executable file on the next dialog."
              );
  FUN_00402fd0("The integration into browsers by using plugins has failed!");
  FUN_00402fd0("Plugins for %s browser have been installed.");
  FUN_00402fd0("Plugins for %s %s browser have been installed.");
  FUN_00402fd0("Plugins for %s browser have been removed.");
  FUN_00402fd0("Please restart the browser to apply changes.");
  FUN_00402fd0("Please locate the browser executable file on the next dialog.");
  FUN_00402fd0("Cannot install idmmzcc.xpi browser extension for Mozilla Firefox!");
  FUN_00402fd0(
              "IDM has been successfully integrated into Mozilla Firefox.\nYou need to restart the Firefox browser to apply changes"
              );
  FUN_00402fd0(
              "IDM has detected that %s %s browser is default browser on your computer. \nWould you like to integrate it with IDM?"
              );
  FUN_00402fd0("Cannot install idmmzcc.xpi browser extension!");
  FUN_00402fd0(
              "%s %s browser will be opened now to install the extension for integration with IDM.\nPlease confirm the installation of idmmzcc.xpi extension from Tonec Inc. on corresponding browser dialog and restart the browser after the installation."
              );
  FUN_00402fd0("Cannot install plugins for %s %s browser!");
  FUN_00402fd0(
              "IDM has been successfully integrated into Internet Explorer.\nYou need to restart IE to apply changes."
              );
  FUN_00402fd0(
              "IDM detected that %s %s browser had been also installed on your computer. \nWould you like to integrate it with IDM as well?"
              );
  FUN_00402fd0(
              "The selected browsers will be opened now to install the extension for integration with IDM.\nPlease confirm the installation of idmmzcc.xpi extension from Tonec Inc. on corresponding browser dialog and restart the browser after the installation."
              );
  FUN_00402fd0(
              "Plugins for the selected browsers have been installed.\nPlease restart the browsers to apply changes."
              );
  FUN_00402fd0("IDM has been already integrated into Internet Explorer!");
  FUN_00402fd0(
              "IDM cannot find this browser in basic integration database. You should try advanced browser integration to catch downloads from this browser."
              );
  FUN_00402fd0("Cannot install plugins for browsers with corresponding type of integration!");
  FUN_004eb420();
  FUN_00402fd0("Browser");
  FUN_00402fd0("Type of integration");
  FUN_00402fd0(&DAT_0068f908);
  FUN_00402fd0("Extension");
  FUN_00402fd0("Plugins");
  FUN_00402fd0("Open with...");
  FUN_00402fd0("Locate the browser");
  FUN_00402fd0("IDM is downloading this file now!");
  FUN_00402fd0("Duplicate download link");
  FUN_00402fd0("Show a dialog and ask what to do.");
  FUN_00402fd0("Add the duplicate with a numbered file name");
  FUN_00402fd0("Add the duplicate and overwrite the existing file");
  FUN_00402fd0("If existing file is complete, show download complete dialog; otherwise resume it.");
  FUN_00402fd0("Internet Explorer and IE-based browsers (AOL, MSN, Avant, etc)");
  FUN_00402fd0("Netscape 8");
  FUN_00402fd0("Netscape 7.xx and 6.xx");
  FUN_00402fd0("You may drag & drop this file");
  FUN_00402fd0(
              "Drag this icon to start the drag and drop operation for the downloaded file.\r\nPress F1 to read more."
              );
  FUN_00402fd0("Drag and drop the downloaded file");
  FUN_00402fd0(
              "This feature is not available for Windows Vista, but it should be implemented soon. Please check for IDM updates."
              );
  FUN_00402fd0(
              "You\'ll need to provide administrator permissions to save files in the selected location"
              );
  FUN_00402fd0("You\'ll need to provide administrator permissions to perform this operation");
  FUN_00402fd0(
              "The IDM Installation program must reboot your computer to replace files that were in use. Press OK button to reboot your computer now"
              );
  FUN_00402fd0("Confirm reboot");
  FUN_00402fd0(
              "Internet download manager has been installed successfully. It will start automatically and take over downloads from your browser."
              );
  FUN_00402fd0(
              "You may also start it from \"Start -> All Programs -> Internet Download Manager\" menu."
              );
  FUN_00402fd0(
              "The requested file looks like html web page.\r\n\r\nThe server sent an html page on the second request. Some servers don\'t allow requesting the same file twice. Since you have \"Start downloading immediately while displaying Download File Info dialog\" option turned off, IDM sent the first request to find out file type and size, and sent the second request when you pressed on \"Start download\". \r\n\r\nTry to turn on \"Start downloading immediately while displaying Download File Info dialog\" option in \"Options->Downloads\", refresh the page in your browser, and try to download this file again."
              );
  FUN_00402fd0(
              "Some sites do not allow requesting the same file twice.\r\n\r\nThe browser made the first request of \"%s\" file and then when IDM tried to request the same file again, the site sent a web page to IDM instead of the download.\r\n\r\nTo take over this download you can set a special key in \"IDM Options->General->Keys...\". Press and hold this key while clicking on download link/button so that IDM could take over the download before the first browser request. Note that \"Ins\" key should work for most browsers.\r\n"
              );
  FUN_00402fd0("Download limits");
  FUN_00402fd0("Queue # %ld");
  FUN_00402fd0("Main download queue");
  FUN_00402fd0("Synchronization queue");
  FUN_00402fd0("Create new queue");
  FUN_00402fd0("Move to queue");
  FUN_00402fd0("Add to queue");
  FUN_00402fd0("Start now");
  FUN_00402fd0(&DAT_006a10f4);
  FUN_00402fd0("Edit queue");
  FUN_00402fd0("Schedule");
  FUN_00402fd0("Do you really want to delete %s?");
  FUN_00402fd0("Start %s");
  FUN_00402fd0("Stop %s");
  FUN_00402fd0("IDM Queues");
  FUN_00402fd0(
              "The pictures in \"Q\" column show which queue files belong to.\r\nYou can move files from one queue to another with file context menu.\r\nFor more information about queues, please refer to Scheduler help file."
              );
  FUN_00402fd0(
              "Would you like to download and install \"IDM Scheduler and queues\" help file (about 100 KB) from IDM home site ?"
              );
  FUN_00402fd0("Queues");
  FUN_00402fd0(
              "\"IDM CC\" extension for Mozilla based browsers has changed in this version of IDM. You will need to reinstall the extension to use new features. Do you want to reinstall the extension for %s %s browser?"
              );
  FUN_00402fd0(
              "\"IDM CC\" extension for Mozilla based browsers has changed in this version of IDM. You will need to reinstall the extension to use new features. Do you want to install the new  \"IDM CC\" extension for browsers which have an old \"IDM CC\" extension?"
              );
  FUN_00402fd0(
              "The data transfer has been interrupted and the server does not support \"resume\". It\'s only possible to download this file from the beginning.\r\n\r\nThere are several possible ways to solve the problem:\r\n1. It\'s possible that the server had temporary problems. Try to resume this download at a later time.\r\n2. You can leave this file alone in the queue (create another new queue) and start the queue. In this case IDM will retry to download the file from the beginning until it downloads the file successfully.\r\nTo achieve the best results, you should also set 1 connection for this site in the list of exceptions in IDM \"Options->Connection\" tab.\r\n"
              );
  FUN_00402fd0("Download with IDM");
  FUN_00402fd0("Download FLV video with IDM");
  FUN_00402fd0("Download last requested FLV video");
  FUN_00402fd0("Choose from 10 last requested FLV videos");
  FUN_00402fd0("Download last requested FLV video with IDM");
  FUN_00402fd0("Download FLV videos with IDM from 10 last requested");
  FUN_00402fd0("Download all links with IDM");
  FUN_00402fd0("Download FLV video content with IDM");
  FUN_00402fd0("Firefox and other Mozilla based");
  FUN_00402fd0("Please restart Internet Explorer to apply changes");
  FUN_00402fd0("Changing IE context menu");
  FUN_00402fd0("Exit now");
  FUN_00402fd0("Exit IDM");
  FUN_00402fd0(
              "Internet Download Manager has finished queue processing. According to scheduler settings IDM is going to exit. But you have other downloads in progress which will be stopped on exit. Press Cancel button not to exit."
              );
  FUN_00402fd0(
              "You have cancelled similar downloads.\n\nIf IDM always takes over such downloads by mistake, you may add this site or a pattern for similar addresses to IDM list of exceptions:"
              );
  FUN_00402fd0("Don\'t catch downloads from similar addresses by using this pattern:");
  FUN_00402fd0("(Limited)");
  FUN_00402fd0("Speed Limiter");
  FUN_00402fd0("Turn on");
  FUN_00402fd0("Turn off");
  FUN_00402fd0("Settings...");
  FUN_00402fd0(
              "A CRC error occurred while downloading. That means that you had some problem with your hard disk. You should scan your disk for errors by using \"Error-checking\" on disk properties->tools Windows dialog."
              );
  FUN_00402fd0(
              "You\'ll need to provide administrator permissions to register IDM for all users on this computer"
              );
  FUN_00402fd0(
              "The selected downloads are not complete. Are you sure you want to delete them from IDM list of downloads?"
              );
  FUN_00402fd0(
              "The selected download is not complete. Are you sure you want to delete it from IDM list of downloads?"
              );
  FUN_00402fd0(
              "There are incomplete downloads among selected ones. Are you sure you want to delete them from IDM list of downloads as well?"
              );
  FUN_00402fd0("Are you sure you want to delete all completed downloads from IDM list of downloads?"
              );
  FUN_00402fd0("Confirm deletion of downloads");
  FUN_00402fd0("video");
  FUN_00402fd0("audio");
  FUN_00402fd0("Download this %s");
  FUN_00402fd0(
              "Note: If you uncheck a file type on the list above and if the file type is in the list on \"Options->File Types\" tab, downloads of this file type are captured by IDM automatically and won\'t be played in the web-player. If you want to prevent it, check the box below:"
              );
  FUN_00402fd0(
              "You can use integration into this browser after enabling \"Advanced browser integration\" feature on this dialog"
              );
  FUN_00402fd0("Cannot resume downloading the file:");
  FUN_00402fd0("IDM is waiting for the new address to resume the file:");
  FUN_00402fd0(
              "It\'s possible that the address of the file is not valid anymore or your session has expired."
              );
  FUN_00402fd0("Is this the new address for your expired download?");
  FUN_00402fd0("Waiting for new data");
  FUN_00402fd0("Refresh download address");
  FUN_00402fd0(
              "IDM is waiting for a new address for another expired download. Only one expired download should use this feature at the same time."
              );
  FUN_00402fd0(
              "IDM will open a web page in your browser where it captured this download. Please start the download of the same file from your browser again, and IDM will try to capture a new address or new session data to resume this download"
              );
  FUN_00402fd0("New link address for this file was received successfully.");
  FUN_00402fd0(
              "Please start the download of the same file from your browser manually, and IDM will try to capture a new address or new session data to resume this download"
              );
  FUN_00402fd0(
              "Try to refresh the download session and get a new address by using \"Refresh download address\" menu item of the right click file menu in IDM download list. \r\nOr Try to delete this download, then start it again from your browser to download it with IDM from the beginning."
              );
  FUN_00402fd0("Downloading owner web page to refresh link address");
  FUN_00402fd0(
              "The web site sent a web page instead of a file when IDM requested this file second time. Probably this site uses temporary links and does not allow requesting the same address twice.\r\n\r\nTry to delete this download, then start it again from your browser and, when IDM captures it, download it without a stop/pause."
              );
  FUN_00402fd0("Checking address: %s");
  FUN_00402fd0(
              "You have not checked any file to download!\r\nIf you don\'t want to download anything please close \"%s\" dialog using \"Cancel\" button."
              );
  FUN_00402fd0("Adding files to download with IDM");
  FUN_00402fd0("You may turn on this panel in \"Options->Downloads\"");
  FUN_00402fd0("Don\'t add to any queue");
  FUN_00402fd0("Several URLs found in clipboard. Do you want to download them?");
  FUN_00402fd0("Batch download from clipboard.");
  FUN_00402fd0("Cannot find any web address in the clipboard!");
  FUN_00402fd0(
              "Your license for Internet Download Manager has expired.\r\nIDM has stopped working. Please obtain a new license to use IDM"
              );
  FUN_00402fd0(
              "Attention:\r\nYour license for IDM will expire in %ld days.\r\n\r\nPlease note that \"Registration->Registration\" menu item is disabled now,\r\nbut it will be enabled once your license expires"
              );
  FUN_00402fd0("You have NOT checked any days of week when the scheduler should start!");
  FUN_00402fd0("Start daily");
  FUN_00402fd0(
              "You have entered a space before the user name. If the space was entered by mistake, would you like IDM to delete the space?"
              );
  FUN_00402fd0(
              "You have entered a space after the user name. If the space was entered by mistake, would you like IDM to delete the space?"
              );
  FUN_00402fd0(
              "You have entered a space before the password. If the space was entered by mistake, would you like IDM to delete the space?"
              );
  FUN_00402fd0(
              "You have entered a space after the password. If the space was entered by mistake, would you like IDM to delete the space?"
              );
  FUN_00402fd0("Downloading until a match with a frame in the next file part...");
  FUN_00402fd0("For web-players");
  FUN_00402fd0("For selected files");
  FUN_00402fd0(
              "(Currently this feature works for Internet Explorer, IE based browsers, Firefox and Mozilla-based browsers)"
              );
  FUN_00402fd0("Download selected file(s) with IDM");
  FUN_00402fd0(
              "IDM cannot find a component that installs/uninstalls advanced browser integration. Please reinstall IDM."
              );
  FUN_00402fd0("Would you like to know how to download files from rapidshare with IDM?");
  FUN_00402fd0("Would you like to know how to download from file sharing sites with IDM?");
  FUN_00402fd0("IDM will download a web-page instead of a file.");
  FUN_00402fd0("Duration");
  FUN_00402fd0(
              "Please note that if you have several users on your computer, advanced browser integration will be turned off for all users."
              );
  FUN_00402fd0(
              "After integrating Internet Download Manager into the new application you may turn off \"Detect New Applications\" option for better performance."
              );
  FUN_00402fd0(
              "This is a BETA version of Internet Download Manager.\r\nIt will install a new network driver, which significantly improves integration with web players and changes old integration with several browsers like Chrome or Opera. If you encounter any problems when connecting to the Internet, for example, a system freezing, or crashes, etc., please open IDM options and turn off \"Use advanced browser integration\" checkbox on General tab. It will turn off the new driver."
              );
  FUN_00402fd0("An unknown error occurred during download. Please try again.");
  FUN_00402fd0("An error occurred during download. Please try again.");
  FUN_00402fd0(
              "Your Serial Number for Internet Download Manager has been blocked because of different registrations. IDM is exiting..."
              );
  FUN_00402fd0(
              "IDM found out that you computer might not have a player that would play %s videos. You will need to install any %s player.\r\n\r\nWould you like to read about a %s player?"
              );
  FUN_00402fd0("%s video has been downloaded");
  FUN_00402fd0("Would you like to know how to download files from %s site with IDM?");
  FUN_00402fd0(
              "You have placed IDM\'s executable file on the desktop. IDM cannot work correctly without its other files, please move IDMan.exe back to programs, and create a shortcut to IDM on Desktop instead. To create a shortcut, open IDM folder in Programs, right click on IDMan.exe file and use \"Send To -> Desktop (create shortcut)\" popup menu item."
              );
  FUN_00402fd0("IDM executable is on desktop");
  FUN_00402fd0(
              "Note 2:\r\nSometimes when you click on a link, your browser requests other files AT THE SAME TIME like Java scripts, web pages, pictures, etc. When you press and hold down a special key, IDM intercepts these files, and the browser may not request the necessary file because it will not run a Javascript, which has been intercepted by IDM erroneously. If you want to intercept and download only the necessary files with IDM, please turn on the option below:"
              );
  FUN_00402fd0(
              "You have an obsolete %s browser integration, or %s integration is not installed. Would you like to read how to fix it?"
              );
  FUN_00402fd0(
              "IDM shows a download panel instead of capturing downloads in web players when %s integration is working correctly."
              );
  FUN_00402fd0(
              "Internet Download Manager will install a new network driver, which significantly improves integration with web players and changes old integration with several browsers like Chrome or Opera. If you encounter any problems with your browser, please open IDM options and turn off \"Use advanced browser integration\" checkbox on General tab. It will turn off the new driver."
              );
  FUN_00402fd0(
              "IDM has intercepted this file from web media player because of the disabled download panel for the respective file type in IDM Options.\r\n\r\nIf you want IDM to refrain from intercepting such files please go to the \"General\" tab in IDM Options, click \"Edit...\" button for download panels in browsers and turn on \"Don\'t capture downloads from web-players automatically\" checkbox in the pop-up window."
              );
  FUN_00402fd0(
              "IDM cannot engage Advanced Browser Integration because of any of the following reasons:\r\n1. You do not have Administrator rights or you did not allow IDM helper program to execute as Administrator.\r\n2. You pressed Cancel button during IDM network driver installation.\r\n3. You blocked IDM network driver startup or installation by means of the third-party security software or antivirus."
              );
  FUN_00402fd0(
              "IDM cannot find %ld file(s) that are necessary for browser and system integration. Please reinstall IDM"
              );
  FUN_00402fd0("What is the \"%s\"?");
  FUN_00402fd0(
              "%s is a product key of Internet Download Manager.\r\nThis is a unique alphanumeric code for each user that looks like\r\n xxxxx-xxxxx-xxxxx-xxxxx\r\n\r\nYou will receive your Serial Number ONLY after IDM purchase.\r\nPlease press \"%s\" button to buy IDM."
              );
  FUN_00402fd0("These settings are unavailable for the downloads which are processed from IDM queue"
              );
  FUN_00402fd0("These settings are unavailable when \"%s\" is turned on");
  FUN_00402fd0(
              "Internet Download Manager has finished downloading a file. According to your settings for this download IDM is going to exit. But you have other downloads in progress which will be stopped on exit. Press Cancel button not to exit."
              );
  FUN_00402fd0(
              "Internet Download Manager has finished downloading a file. According to your settings for this download IDM is going to hang up your modem."
              );
  FUN_00402fd0(
              "Internet Download Manager has finished downloading a file. According to your settings for this download your computer is shutting down."
              );
  FUN_00402fd0(
              "IDM cannot download this protected stream for legal reasons. The download of such streams is not supported because IDM may not bypass the technological measures which are made for the protection of audio, video and data content.\r\n\r\nIt\'s possible to turn off download panel for the protected content that IDM may not download in \"IDM Options -> General tab\""
              );
  FUN_00402fd0(
              "It\'s possible that you need to change VPN connection that is set in IDM options, or turn off  \"Use Windows Dial Up / VPN Networking\" checkbox in IDM options -> \"Dial Up / VPN\" tab"
              );
  FUN_00402fd0("Error connecting to VPN");
  FUN_00402fd0("Import links to IDM");
  FUN_00402fd0("Cannot connect to socks server, authorization required");
  FUN_00402fd0("Socks server cannot connect to %s ");
  FUN_00402fd0("Cannot connect via socks server, unknown socks type");
  FUN_00402fd0("Cannot connect via socks server, authorization failed!");
  FUN_00402fd0("Cannot connect to the socks server %s");
  FUN_00402fd0("Proxy / socks configuration");
  FUN_00402fd0("Proxy/Socks");
  FUN_00402fd0(
              "IDM cannot engage Advanced Browser Integration because \"Base Filtering Engine\" Windows service is not running. Please right-click the Computer icon, select \"Manage\", navigate to \"Services and Applications -> Services\", then find \"Base Filtering Engine\" in the list and right-click on it to open Properties. Change Startup type to \"Automatic\", click \"Start\" and confirm changes.\r\n\r\nClick OK to visit a web page for additional information."
              );
  FUN_00402fd0(
              "IDM cannot engage Advanced Browser Integration because \"Base Filtering Engine\" Windows service is missing. This could happen because of your system being damaged by a computer malware.\r\n\r\nClick OK to open a web page with recovery instructions."
              );
  FUN_00402fd0(
              "Please note that according to IDM license that you have accepted during IDM installation, now you should either uninstall IDM or purchase it"
              );
  FUN_00402fd0("Recent...");
  FUN_00402fd0(
              "Your browser may not open IDM website because an important system file is damaged on your computer. Repair this file?"
              );
  FUN_00402fd0(
              "IDM cannot check for updates because an important system file is damaged on your computer. Repair this file?"
              );
  FUN_00402fd0(
              "In order to update a part of IDM extension for Firefox please close Firefox (or SeaMonkey) and then press \"Retry\" button\r\nOr you will need to restart your computer later"
              );
  FUN_00402fd0("IDM needs to download main translation file for your language from IDM home site");
  FUN_00401980(DAT_007676ec,DAT_00765d80);
  FUN_00402fd0(
              "IDM started to download the file using one connection because of data corruption when newly received data in several connections did not match the same data downloaded earlier.\r\nDo you want to read about possible reasons of data corruption?"
              );
  FUN_00402fd0(
              "IDM started to download the file from the beginning since data was corrupted because newly received data did not match the same data downloaded earlier.\r\nDo you want to read about possible reasons of data corruption?"
              );
  FUN_00402fd0(
              "IDM extension has been successfully installed into (or updated in) Google Chrome browser.\nYou need to restart the Chrome to apply changes"
              );
  FUN_00402fd0(
              "Proxy authorization is required for this proxy server. Or please turn off \"Use automatic configuration script\" checkbox in IDM Options->Proxy/Socks"
              );
  FUN_00402fd0(
              "IDM has detected that you are using the following automatic proxy configuration script in Internet Explorer:\r\n%s\r\n\r\nWould you like to use the same proxy settings in IDM?"
              );
  FUN_00402fd0(
              "This is a live stream broadcast. If you stop it, you will not be able to resume downloading from the current position.\r\nWould you like to stop and save the broadcast as a complete file?"
              );
  FUN_00402fd0("Live stream saving");
  FUN_00402fd0("Downloading of proxy script failed");
  FUN_00402fd0(
              "Would you like to read detailed instructions how to configure your firewall to work with IDM?"
              );
  FUN_00402fd0(
              "Advanced Browser Integration cannot be enabled because a flawed version of Kaspersky TDI driver is installed!\r\nIf automatically updating your Kaspersky product does not help, please contact Kaspersky support to get an update for the \"kltdi\" driver."
              );
  FUN_00402fd0("An error has occurred during assembling the file");
  FUN_00402fd0(
              "This version of IDM does not support this type of downloading. Try to update IDM to the latest version."
              );
  FUN_00402fd0(
              "IDM extension has been successfully installed into (or updated in) Google Chrome browser.\r\nYou must enable \"IDM Integration module\" extension in Google Chrome settings if you want IDM to work with Chrome properly"
              );
  FUN_00402fd0(
              "1. Please RESTART Google Chrome and press on Chrome menu (arrow 1 on the image)\r\n2. Select \"More tools\" menu item (arrow 2 on the image).\r\n3. Select \"Extensions\" menu item (arrow 3 on the image). \r\n4. Check that \"IDM Integration Module\" extension is enabled (arrow 4 on the image).\r\n5. If you use incognito mode in Chrome, you need to enable \"Allow in incognito\" checkbox (arrow 5 on the image)."
              );
  FUN_00402fd0("Low speed: Dial Up modem / ISDN / Bluetooth / Mobile Edge / IrDA");
  FUN_00402fd0("Medium speed: ADSL / DSL /  Mobile 3G / Wi-Fi / Bluetooth 3.0 / other");
  FUN_00402fd0("High speed: Direct connection (Ethernet/Cable) / Wi-Fi / Mobile 4G / other");
  FUN_00402fd0(
              "Please use a high number of connections with caution. Some servers may have specific limitations for a number of connections from one client. When the number of connections is high, the servers can put your computer to a black list, especially when you are downloading several files at the same time with a high number of connections.\r\n\r\nMoreover, for some connection types, or for low performance computers or routers, the download speed may decrease when you increase the number of connections."
              );
  FUN_00402fd0("There is only a folder name in your selection. Please enter a name for the file.");
  FUN_00402fd0(
              "The size of \"%s\" file is %.1f GBytes.\r\nThe drive \"%s\" where you want to save this file has %s file system. The files larger than %d GBytes cannot be written on this file system.\r\n\r\nPlease choose a folder for this file on another drive with NTFS file system"
              );
  FUN_00402fd0(
              "The size of \r\n%s\r\n file is %.1f GBytes.\r\nIDM\'s temporary directory for storing file parts during download is located on drive \"%s\", this drive has %s file system. The files larger than %d GBytes cannot be written on this file system.\r\n\r\nTo continue downloading, please change temporary directory location to another drive in \"Options->Save to\" and then resume downloading"
              );
  FUN_00402fd0(
              "Check for updates failed, because update information is invalid.\r\nIf you use wi-fi that requires logging in in the browser, please log in before checking for updates"
              );
  FUN_00402fd0(
              "Some application has damaged IDM integration into browsers\r\n\r\nYou will need to reinstall IDM to restore it. Or try to update IDM by using \"Help->Check for updates\" IDM menu item\r\nAlso verify that IDM integration into your browser is turned on in IDM Options -> General tab."
              );
  FUN_00402fd0("Find file");
  FUN_00402fd0("No such files found");
  FUN_00402fd0("The search is finished");
  FUN_00402fd0(&DAT_00696c08);
  FUN_00402fd0("Move/Rename");
  FUN_00402fd0(
              "IDM could not start Update application. This may be due to incorrect antivirus software settings for the folder containing IDM service files.\r\n\r\nIDM will now open this folder, and you can run \"idmupdt.exe\" file manually.\r\n\r\nOr you can download the latest version of IDM from IDM home site, and install it over your current version"
              );
  FUN_00402fd0("Select a Folder");
  FUN_00402fd0(
              "IDM has downloaded %s file that uses a new codec that is not installed on your computer.\r\nPlease read more information on IDM home site to play such files in your player."
              );
  FUN_00402fd0(
              "Several IDM modules have not been updated. Please reinstall the latest version of IDM, reboot your computer, and try again to download this file"
              );
  FUN_00402fd0(
              "IDM will not be able to wake up computer from sleep to start your download queue at a scheduled time."
              );
  FUN_00402fd0("Please note that when you switch to Metro mode, desktop may go to a sleeping mode.")
  ;
  FUN_00402fd0(
              "If you want IDM to download the queue at a scheduled time, please enable wake timers in Windows settings.\r\nOpen \"Control Panel\\Power Options\\Change Plan Settings\\Change advanced power settings\", then open \"Sleep\\Allow wake timers\" node and set it to \"Enable\".\r\n\r\nDo you want to see how to do it on screenshots?"
              );
  FUN_00402fd0(
              "Please note that this version and all previous versions of IDM may not use \"hosts\" file when they connect to IDM home site. When you change this file, you close the access to IDM website for your web browser only. Once you press on OK button, IDM will check for updates successfully even with your damaged \"hosts\" file."
              );
  FUN_00402fd0(
              "Your browser may not open IDM website because an important system file is damaged on your computer. IDM will repair this file now."
              );
  FUN_00402fd0(
              "The serial number for IDM that you entered is used on another computer and connected to a different e-mail address"
              );
  FUN_00402fd0(
              "The serial number for IDM\r\n%s\r\nthat you entered is used on another computer.\r\n\r\nIf you want to use this serial number on this computer, then you need to uninstall IDM from another computer (or all other computers) where this serial number was used earlier. If you are using the same serial number on different computers at the same time, this serial number will be blocked automatically.\r\n\r\nWould you like to continue IDM registration with\r\n%s\r\nserial number?"
              );
  FUN_00402fd0(
              "You registered IDM with the serial number below\r\n%s\r\n\r\nThis serial number is blocked on IDM servers. IDM is exiting..."
              );
  FUN_00402fd0("Would you like to read more?");
  FUN_00402fd0(
              "The digital signature of IDM update file is invalid. It\'s possible that the update file is corrupted.\r\nYou can download a new version of IDM from IDM home site and install it over your current version manually"
              );
  FUN_00402fd0(
              "IDM cannot download the update file.\r\nYou can download a new version of IDM from IDM home site and install it over your current version manually"
              );
  FUN_00402fd0("This license expires on  %s");
  FUN_00402fd0(
              "IDM has found unreferenced and not deleted parts of old downloads in IDM temporary folder\r\n%s\r\nWould you like IDM to delete them to free your disk space?"
              );
  FUN_00402fd0("Your serial number starts with:  %s...............");
  FUN_00402fd0(
              "Do not show \"Download this audio\" button if audio file size is below the following minimum size in kilobytes"
              );
  FUN_00402fd0(
              "Do not show \"Download this video\" button if media file size is below the following minimum size in kilobytes"
              );
  FUN_00402fd0(
              "Note: This limitation works only if the size of media file is known. There are situations when only duration is known, or nothing is known until file is fully downloaded. For such situations this limitation of file size will not work."
              );
  FUN_00402fd0(
              "It\'s possible that you have obsolete Windows XP cryptographic libraries.\r\nWould you like to read more?"
              );
  FUN_00402fd0("Shut down");
  FUN_00402fd0("Sleep");
  FUN_00402fd0("Hibernate");
  FUN_00402fd0(
              "Internet Download Manager has finished queue processing. According to scheduler settings your computer is going to sleep mode."
              );
  FUN_00402fd0(
              "Internet Download Manager has finished processing of grabber project. According to project scheduler settings your computer is going to sleep mode."
              );
  FUN_00402fd0(
              "Internet Download Manager has finished downloading a file. According to your settings for this download your computer is going to sleep mode."
              );
  FUN_00402fd0(
              "Internet Download Manager has finished queue processing. According to scheduler settings your computer will hibernate."
              );
  FUN_00402fd0(
              "Internet Download Manager has finished processing of grabber project. According to project scheduler settings your computer will hibernate."
              );
  FUN_00402fd0(
              "Internet Download Manager has finished downloading a file. According to your settings for this download your computer will hibernate."
              );
  FUN_00402fd0("Sleep mode");
  FUN_00402fd0("Sleep now");
  FUN_00402fd0("Hibernate now");
  FUN_00402fd0("Socks server address");
  FUN_00402fd0("Proxy server address");
  FUN_00402fd0(
              "IDM integration into Firefox 53 and later will work only when \"Use advanced browser integration\" is turned on in IDM Options -> General tab"
              );
  FUN_00402fd0(
              "IDM cannot detect the URL of the Proxy Auto-Configuration file for this network!\r\n\r\nIf you do not have automatic configuration set up for your network, you should not use this option to avoid delays when IDM is trying to find the proxy pack file."
              );
  FUN_00402fd0(
              "A new \"IDM integration module\" extension for Edge has been released. This extension is necessary for the full-fledged integration into Microsoft Edge browser. IDM will now start the installation of this extension from Microsoft store.\r\nMicrosoft Store dialog to install IDM extension will appear when you close this dialog.\r\nPlease press on \"Get\" button on Microsoft Store dialog as it\'s illustrated on the image below."
              );
  FUN_00402fd0("Your IDM license was extended until %s");
  FUN_00402fd0("Improve capture of downloads");
  FUN_00402fd0(
              "IDM will send a report about a problem with repeated download requests from %s web site, and the rules to process downloads from this web site without special keys should be added to IDM servers in some time.\r\n\r\nIt\'s also possible that IDM servers already have a solution to download from this web site, then IDM will update its database after sending this report."
              );
  FUN_00402fd0(
              "The rules to process downloads from %s web site have been updated.\r\nNow downloads from this website can be intercepted without the usage of special keys.\r\n\r\nPlease note that old downloads that were added before the update will not be resumed, you need to delete them and take them over from your browser again."
              );
  FUN_00402fd0("Not modified");
  FUN_00402fd0(
              "IDM intercepted a download that according to %s web site settings can be requested only once, therefore IDM tries to request it before the browser. But there is no enough data in the intercepted request to download this file.\r\n\r\nIt\'s possible that you add a download to browser using an unconventional way bypassing web page processing on a sharing web site. In this case you need to start over with a public link to a download and pass all necessary steps so that intercepted request will have all necessary data.\r\n\r\nIf it\'s not a download and IDM intercepted it by mistake, please set a special key (\"Del\" should work for most browsers) in IDM Options -> General -> \"Customize keys to prevent or force downloading with IDM\" and hold it while clicking on a link to prevent downloading with IDM"
              );
  FUN_00402fd0(
              "The file with download rules that was downloaded from IDM servers has a new format.\r\nPlease update IDM by using \"Help->Check for updates\" IDM menu item so that IDM could use these rules."
              );
  FUN_00402fd0(
              "This problem may occur because \"IDM integration module\" extension is disabled or not installed in your browser."
              );
  FUN_00402fd0("Please install extension for Pale Moon browser from IDM website.");
  FUN_00402fd0(
              "Please note that your browser uses an extension for setting VPN connections. Most likely IDM can\'t download the file because the browser uses different proxy/socks for request of this file and the website declines direct requests from IDM. IDM could not detect the proxy or socks server used by this extension.\r\n\r\nTry to turn off this extension in your browser and try to intercept this download again. If you cannot intercept and download the file without VPN extension, try another VPN extension."
              );
  FUN_00402fd0("Or try to to intercept this download from another browser");
  FUN_00402fd0(
              "If you would like to run antivirus software on download completion, you should type or browse the complete path to its executable file. If IDM knows about necessary parameters for a given executable file name, it will set them automatically. Otherwise, you will have to set them manually in \"Command line parameters\" according to antivirus software documentation. The scanned file among different parameters should be written as\r\n[File]\r\n(you should type square brackets as well). If no parameters are set, IDM will execute antivirus software appending the complete path of a downloaded file."
              );
  FUN_00402fd0(
              "Note that all modern antivirus software have \"real-time protection\" feature that checks all operations with files on your computer, and it includes all files downloaded in IDM. Thus, you need to add antivirus software scanner to IDM settings only when real-time protection is turned off in antivirus software settings.\r\nAlso, in some cases, real time protection may significantly slow down the assembly of files made by IDM. Then you need to add IDMan.exe process to exceptions according to antivirus software documentation, and set up antivirus scan on this dialog."
              );
  FUN_00402fd0(
              "The video has not been mixed with the audio because the audio stream has a different format. The audio file will be saved separately as\r\n%s"
              );
  FUN_00402fd0("On Double click");
  FUN_00402fd0(
              "You cannot open this file because its download is not complete.\r\nYou may change double click action to \"Properties\" in right click menu"
              );
  FUN_00402fd0(
              "Such problem is possible also when one of IDM files with download rules for specific web sites is deleted, or installed incorrectly. It could happen because of invalid IDM installation.\r\nPlease download and install the latest IDM version from official IDM web site."
              );
  FUN_00402fd0(
              "IDM found out that some application had disabled loading of IDM module that is required for IDM integration into browser.\r\nIDM will try to restore the normal operation of this module now. You will need to confirm running \"Internet Download Manager assistant\" with administrative rights on the next step."
              );
  FUN_00402fd0(
              "IDM has unblocked the module for integration into browsers. However, it may be necessary to restart the computer for the changes to take effect."
              );
  FUN_00402fd0(
              "This is a global Windows setting, so some applications using the Windows Internet Options may work with errors\r\nAlso you will need to confirm running \"Internet Download Manager assistant\" with administrative rights on the next step"
              );
  FUN_00402fd0("Cannot load ssl library. Please try to reinstall IDM.");
  FUN_00402fd0("Error on loading ssl library. Please try to reinstall IDM.");
  FUN_00402fd0("Unknown error on loading ssl library.");
  FUN_00402fd0(
              "You must install \"IDM Integraton Module\" extension (add-on) to turn on IDM integration into Firefox browser. IDM will open add-on installation page. Then please press \"+ Add to Firefox\" button, and confirm the installation."
              );
  FUN_00402fd0(
              "This is 30 day trial version of Internet Download Manager. Do you want to register your copy of IDM now?"
              );
  FUN_00402fd0(
              "These changes will be applied only after restarting IDM. Do you want to restart IDM now?"
              );
  FUN_00402fd0(
              "IDM will use Dark Mode when it is enabled in Windows settings\r\n(Right click on Desktop -> Personalize -> Colors -> Dark app mode)"
              );
  FUN_00402fd0("IDM is registered");
  FUN_00402fd0("Already purchased?");
  FUN_00402fd0("You have <font color=\"red\">%d days left</font> to use Internet Download Manager");
  FUN_00402fd0(
              "Don\'t wait for trial period expiration. Choose a special price option below to buy IDM license right now"
              );
  FUN_00402fd0("Internet Download Manager free trial period has expired");
  FUN_00402fd0(
              "You may buy IDM to continue using it, or uninstall it. Click on special price buttons below to buy IDM right now"
              );
  FUN_00402fd0("This is a lifetime license");
  FUN_00402fd0("IDM Search settings");
  FUN_00402fd0("The checkbox for at least one of the search targets must be enabled");
  FUN_00402fd0("Parent web page");
  FUN_00402fd0(
              "IDM could not take over the download despite existing special rules of taking over downloads from %s web site. The rules may have to be changed.\r\nDo you want to send the report about this problem to IDM web site?"
              );
  FUN_00402fd0(
              "The font size changes on the main dialog will apply only after restarting IDM. Do you want to restart IDM now?"
              );
  FUN_00402fd0(" Chrome, Edge, IE ");
  FUN_00402fd0("Mixing audio and video streams into one file");
  FUN_00402fd0(
              "Drive %s is not available.\r\nPerhaps this is an external drive. Check the drive availability or select another drive to save the file and click \"Resume\""
              );
  FUN_00402fd0(&DAT_00693ce8);
  FUN_00402fd0("VPN Settings");
  FUN_00402fd0(
              "Because of frequent updates of browsers and changes on web sites, IDM needs to accommodate to these updates and changes, and that\'s why IDM is updated frequently.\r\nThus, it\'s very important to always use the latest version of IDM to keep all IDM functions working.\r\n\r\nAre you sure that you don\'t need to update to the latest version of IDM?"
              );
  FUN_00402fd0(
              "Internet Download Manager cannot turn on its integration into browsers because your device is missing an important Windows update.\r\nWould you like to read how to install this update on our web site?"
              );
  FUN_00402fd0("Extend the availability of Manifest 2 Chrome extensions");
  FUN_00402fd0("You need to restart Google Chrome to apply changes");
  FUN_00402fd0("Extend manifest V2 availability");
  FUN_00402fd0(
              "Google Chrome pushes a new type of extensions called Manifest 3 extensions. Old Manifest 2 extensions will be disabled in Chrome soon.\r\nPlease note that the functionality of Manifest 3 extensions is worse than the current Manifest 2 extensions. Google has limited many old V2 functions in Manifest version 3.\r\nBut there is a way to extend usage of Manifest 2 extensions by adding \"ExtensionManifestV2Availability\" Chrome Enterprise policy, and IDM has already added this policy. \r\nYou may turn it off any time in IDM Options -> General tab. \r\n\r\nYou need to restart Google Chrome to apply changes"
              );
  FUN_00402fd0("Please enter a valid email address.");
  FUN_00402fd0(
              "Please ensure the email address entered is accurate, as it may be used to recover your IDM serial number or access prioritized support."
              );
  FUN_00402fd0("Advanced browser integration is enabled");
  FUN_00402fd0("Restart");
  FUN_00402fd0("Advanced browser integration is disabled");
  FUN_00402fd0("Enable");
  FUN_004e9930();
  local_8 = CONCAT31(local_8._1_3_,4);
  FUN_00402570();
  local_8 = 0xffffffff;
  FUN_00402570();
  ExceptionList = local_10;
  return;
}


