
public class RuntimeError: Exception {
    public readonly Token token;

    public RuntimeError(Token token, string message) : base(message) {
        this.token = token;
    }
}

// this is disgusting so i'm naming it as such. It might be clever if it weren't so horrific
public class Abomination : Exception {
    public object? result;

    public Abomination(object? result) : base() {
        this.result = result;
    }
}