import {
  Button,
  Checkbox,
  Label,
  Select,
  TextInput,
} from "flowbite-react";
import { useState } from "react";
import { Controller, useForm } from "react-hook-form";

interface RestrictionRuleFormData {
  name: string;
  rule_type: "Asn" | "IP" | "IPCidr" | "UserAgent";
  rule_value: string;
  expires_at?: string;
}

interface DefaultValues {
  name: string;
  rule_type: "Asn" | "IP" | "IPCidr" | "UserAgent";
  rule_value: string;
  expires_at?: string | null;
}

interface SubmitData {
  name: string;
  rule_type: "Asn" | "IP" | "IPCidr" | "UserAgent";
  rule_value: string;
  expires_at?: string;
}

type Props =
  | {
      mode: "create";
      onSubmit: (data: SubmitData) => void;
    }
  | {
      mode: "edit";
      defaultValues: DefaultValues;
      onSubmit: (data: SubmitData) => void;
    };

const RULE_TYPE_OPTIONS = [
  { value: "Asn", label: "ASN" },
  { value: "IP", label: "IP Address" },
  { value: "IPCidr", label: "IP CIDR" },
  { value: "UserAgent", label: "User Agent" },
];

const RestrictionRuleForm = (props: Props) => {
  const defaults = props.mode === "edit" ? props.defaultValues : undefined;
  const [neverExpires, setNeverExpires] = useState(
    defaults ? !defaults.expires_at : true,
  );

  const { register, handleSubmit, control, reset } =
    useForm<RestrictionRuleFormData>();

  return (
    <form
      onSubmit={handleSubmit((data) => {
        props.onSubmit({
          name: data.name,
          rule_type: data.rule_type,
          rule_value: data.rule_value,
          expires_at:
            neverExpires || !data.expires_at
              ? undefined
              : new Date(data.expires_at).toISOString(),
        });
        reset();
        setNeverExpires(true);
      })}
    >
      <div className="flex flex-col space-y-4">
        <div>
          <Label>Name</Label>
          <TextInput
            placeholder="Rule name..."
            required
            defaultValue={defaults?.name}
            {...register("name", { required: true })}
          />
        </div>
        <div>
          <Label>Rule Type</Label>
          <Controller
            name="rule_type"
            control={control}
            rules={{ required: true }}
            defaultValue={defaults?.rule_type}
            render={({ field }) => (
              <Select
                required
                value={field.value}
                onChange={(e) => field.onChange(e.target.value)}
              >
                <option value="">Select rule type...</option>
                {RULE_TYPE_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </Select>
            )}
          />
        </div>
        <div>
          <Label>Rule Value</Label>
          <TextInput
            placeholder="Rule value..."
            required
            defaultValue={defaults?.rule_value}
            {...register("rule_value", { required: true })}
          />
        </div>
        <div>
          <div className="flex items-center space-x-2 mb-3">
            <Checkbox
              id="never-expires"
              checked={neverExpires}
              onChange={(e) => setNeverExpires(e.target.checked)}
            />
            <Label htmlFor="never-expires">Never expires</Label>
          </div>
          {!neverExpires && (
            <div>
              <Label>Expires At</Label>
              <TextInput
                type="datetime-local"
                defaultValue={
                  defaults?.expires_at
                    ? new Date(defaults.expires_at).toISOString().slice(0, 16)
                    : ""
                }
                {...register("expires_at")}
              />
            </div>
          )}
        </div>
      </div>
      <Button type="submit" className="mt-4">
        {props.mode === "create" ? "Create Rule" : "Update Rule"}
      </Button>
    </form>
  );
};

export default RestrictionRuleForm;
