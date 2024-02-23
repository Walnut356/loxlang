using System.Linq.Expressions;

namespace Callable;

public interface Any
{
    public object? Call(Interpreter interp, List<object?> args);
    int Arity();
}

public class Anon(int _arity, Func<Interpreter, object> _call) : Any
{
    public int Arity()
    {
        return _arity;
    }

    public object? Call(Interpreter interp, List<object?> args)
    {
        return _call(interp);
    }
}

public class Func(Stmt.Function inner) : Any
{
    public int Arity()
    {
        return inner.parameters.Count;
    }

    public object? Call(Interpreter interp, List<object?> args)
    {
        Env env = new(interp.globals);

        foreach ((var p, var a) in inner.parameters.Zip(args))
        {
            env.Init(p.token, a);
        }

        Env prev = interp.env;

        object? result = null;
        try
        {
            interp.env = env;

            foreach (var stmt in inner.block)
            {
                stmt.Execute(interp);
            }
        }
        catch (Abomination ret)
        {
            result = ret.result;
        }
        finally
        {
            interp.env = prev;
        }
        
        return result;
    }

    public override string ToString()
    {
        return $"<fn {inner.ident.token}>";
    }
}