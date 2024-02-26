using static Utils;

public record Env()
{
    public Env? parent = null;
    Dictionary<string, object?> vars = [];

    public Env(Env parent)
    {
        this.parent = parent;
        vars = [];
    }

    public void Init(string ident, object? val)
    {
        vars[ident] = val;
        // if (vars.TryGetValue(ident, out var _))
        // {
        //     vars[ident] = val;
        // }
        // else
        // {
        //     vars.Add(ident, val);
        // }
    }

    public object? Get(Token ident)
    {
        object? result;
        if (vars.TryGetValue(ident.token, out result))
        {
            return result;
        }
        Env? temp = parent;
        while (temp is not null)
        {
            if (temp.vars.TryGetValue(ident.token, out result))
            {
                return result;
            }
            temp = temp.parent;
        }

        throw new RuntimeError(ident, "Undefined variable '" + ident.token + "'.");
    }

    public object? GetAt(int dist, Token ident) {
        Env env = this!;
        for (int i = 0; i < dist; i++) {
            env = env.parent!;
        }

        return env.Get(ident);
    }

    public void Assign(Token ident, object? val)
    {
        if (vars.TryGetValue(ident.token, out var _))
        {
            vars[ident.token] = val;
            return;
        }
        Env? temp = parent;
        while (temp is not null)
        {
            if (temp.vars.TryGetValue(ident.token, out _))
            {
                temp.vars[ident.token] = val;
                return;
            }
            temp = temp.parent;
        }

        throw new RuntimeError(ident, "Undefined variable '" + ident.token + "'.");
    }

    public void AssignAt(int dist, Token ident, object? val) {
        Env env = this;
        for (int i = 0; i < dist; i++)
        {
            env = env.parent!;
        }

        env.vars[ident.token] = val;
    }

    internal void Assign(string token, object? result)
    {
        throw new NotImplementedException();
    }
}