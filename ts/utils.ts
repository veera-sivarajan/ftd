function console_log(...message: any) {
    if (true) { // false
        console.log(...message);
    }
}

function isObject(obj: object) {
    return obj != null && typeof obj === 'object' && obj === Object(obj);
}

function resolve_reference(reference: string, data: any, value: any) {
    if (reference === "VALUE") {
        return value;
    }
    if (!!data[reference]) {
        return deepCopy(data[reference]);
    }
    let [var_name, remaining] = get_name_and_remaining(reference);
    let initial_value = data[var_name];
    while (!!remaining) {
        let [p1, p2] = split_once(remaining, ".");
        initial_value = initial_value[p1];
        remaining = p2;
    }
    return deepCopy(initial_value);
}


function get_name_and_remaining(name: string): [string, string | null] {
    let part1 = "";
    let pattern_to_split_at = name;
    let parent_split = split_once(name, "#");
    if (parent_split.length === 2) {
        part1 = parent_split[0] + "#";
        pattern_to_split_at = parent_split[1];
    }
    parent_split = split_once(pattern_to_split_at, ".");
    if (parent_split.length === 2) {
        return [part1 + parent_split[0], parent_split[1]];
    }
    return [name, null];
}


function split_once(name: string, split_at: string) {
    const i = name.indexOf(split_at);
    if (i === -1) {
        return [name];
    }
    return [name.slice(0, i), name.slice(i + 1)];
}

function deepCopy(object: any) {
    if (isObject(object)) {
        return JSON.parse(JSON.stringify(object));
    }
    return object;
}

function change_value(function_arguments: (FunctionArgument | any)[], data: {
    [key: string]: any;
}, id: string) {
    for (const a in function_arguments) {
        if (isFunctionArgument(function_arguments[a])) {
            if (!!function_arguments[a]["reference"]) {
                let reference: string = <string>function_arguments[a]["reference"];
                let [var_name, remaining] = (!!data[reference]) ? [reference, null] : get_name_and_remaining(reference);
                if (var_name === "ftd#dark-mode") {
                    if (!!function_arguments[a]["value"]) {
                        window.enable_dark_mode();
                    } else {
                        window.enable_light_mode();
                    }
                } else if (!!window["set_value_" + id] && !!window["set_value_" + id][var_name]) {
                    window["set_value_" + id][var_name](data, function_arguments[a]["value"], remaining);
                } else {
                    set_data_value(data, reference, function_arguments[a]["value"]);
                }
            }
        }
    }
}

function isFunctionArgument(object: any): object is FunctionArgument {
    return (<FunctionArgument>object).value !== undefined;
}

String.prototype.format = function() {
    var formatted = this;
    for (var i = 0; i < arguments.length; i++) {
        var regexp = new RegExp('\\{'+i+'\\}', 'gi');
        formatted = formatted.replace(regexp, arguments[i]);
    }
    return formatted;
};


function set_data_value(data: any, name: string, value: any) {
    if (!!data[name]) {
        data[name] = deepCopy(set(data[name], null, value));
        return;
    }
    let [var_name, remaining] = get_name_and_remaining(name);
    let initial_value = data[var_name];
    data[var_name] = deepCopy(set(initial_value, remaining, value));

    // tslint:disable-next-line:no-shadowed-variable
    function set(initial_value: any, remaining: string | null, value: string) {
        if (!remaining) {
            return value;
        }
        let [p1, p2] = split_once(remaining, ".");
        initial_value[p1] = set(initial_value[p1], p2, value);
        return initial_value;
    }
}

function get_data_value(data: any, name: string) {
    if (!!data[name]) {
        return deepCopy(data[name]);
    }
    let [var_name, remaining] = get_name_and_remaining(name);
    let initial_value = data[var_name];
    while (!!remaining) {
        let [p1, p2] = split_once(remaining, ".");
        initial_value = initial_value[p1];
        remaining = p2;
    }
    return deepCopy(initial_value);
}

function JSONstringify(f: any) {
    if(typeof f === 'object') {
        return JSON.stringify(f);
    } else {
        return f;
    }
}
