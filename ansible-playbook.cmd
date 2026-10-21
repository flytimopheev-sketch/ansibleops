@echo off
echo.
echo PLAY [%~n1] ********************************************
echo.
echo TASK [Gathering Facts] ***************
echo ok: [web1]
echo ok: [web2]
echo ok: [db1]
echo.
echo TASK [Ping hosts] ***************
echo ok: [web1]
echo ok: [web2]
echo ok: [db1]
echo.
echo TASK [Install packages] ***************
echo changed: [web1]
echo changed: [web2]
echo fatal: [db1]: FAILED! => {"msg": "Package not found"}
echo.
echo TASK [Start service] ***************
echo ok: [web1]
echo ok: [web2]
echo skipping: [db1]
echo.
echo PLAY RECAP ********
echo web1 : ok=4 changed=1 unreachable=0 failed=0
echo web2 : ok=4 changed=1 unreachable=0 failed=0
echo db1  : ok=2 changed=0 unreachable=0 failed=1
exit /b 1