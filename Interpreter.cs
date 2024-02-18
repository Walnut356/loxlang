
using System.Data;

class Interpreter
{
    // maybe make this static later? That way it doesn't need to be passed around constantly
    Env env = new();

    public void Interpret(List<Stmt.Any> stmts)
    {
        try
        {
            foreach (var stmt in stmts)
            {
                stmt.Execute(env);
            }
        }
        catch (RuntimeError e)
        {
            Lox.RuntimeError(e);
        }
    }

}