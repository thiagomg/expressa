' Duplo clique abre a IDE com o GTK de deps\ e espera ela fechar.
' O proprio programa sobe o servidor e o encerra ao sair.
Set fso = CreateObject("Scripting.FileSystemObject")
Set sh = CreateObject("Wscript.Shell")
root = fso.GetParentFolderName(WScript.ScriptFullName)
If Right(root, 1) <> "\" Then root = root & "\"

debugApp = root & "target\debug\expressa-aula.exe"
releaseApp = root & "target\release\expressa-aula.exe"

If fso.FileExists(debugApp) And fso.FileExists(releaseApp) Then
  If fso.GetFile(releaseApp).DateLastModified >= fso.GetFile(debugApp).DateLastModified Then
    app = releaseApp
  Else
    app = debugApp
  End If
ElseIf fso.FileExists(releaseApp) Then
  app = releaseApp
ElseIf fso.FileExists(debugApp) Then
  app = debugApp
Else
  MsgBox "Nao achei expressa-aula.exe." & vbCrLf & "Na pasta do projeto, rode:" & vbCrLf & "cargo build -p expressa-aula -p expressa-aula-server", 16, "Expressa Aula"
  WScript.Quit 1
End If

sh.CurrentDirectory = root
Set env = sh.Environment("PROCESS")
env("PATH") = root & "deps\gtk\bin;" & env("PATH")
' Exec devolve o pid. AppActivate(pid) puxa a janela deste processo:
' sem isso a IDE abre maximizada atras da janela atual.
Set proc = sh.Exec("""" & app & """")
For i = 1 To 50
  WScript.Sleep 200
  If sh.AppActivate(proc.ProcessID) Then Exit For
Next
Do While proc.Status = 0
  WScript.Sleep 400
Loop
