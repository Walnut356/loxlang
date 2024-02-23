using System.Diagnostics;
using System.Linq.Expressions;
using static TokenType;

public class Interpreter
{
    // maybe make this static later? That way it doesn't need to be passed around constantly
    public Env globals = new();
    public Env env;
    public Stopwatch clock = new();

    public Interpreter()
    {
        clock.Start();
        globals.Init("now",
            new Callable.Anon(
                0,
                (interp) => (double)interp.clock.ElapsedTicks
            )
        );

        env = globals;

    }

    public void Interpret(List<Stmt.Any> stmts)
    {
        try
        {
            foreach (var stmt in stmts)
            {
                stmt.Execute(this);
            }
        }
        catch (RuntimeError e)
        {
            Lox.RuntimeError(e);
        }
    }

}