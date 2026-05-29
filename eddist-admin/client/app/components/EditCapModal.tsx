import { Button, Label, Modal, ModalBody, ModalHeader, TextInput } from "flowbite-react";
import { useForm } from "react-hook-form";
import BoardMultiSelect from "~/components/BoardMultiSelect";
import { useUpdateCap } from "~/hooks/queries";
import type { Cap } from "~/types/entities";

interface EditCapModalProps {
  open: boolean;
  selectedCap: Cap;
  setOpen: (open: boolean) => void;
}

const EditCapModal = ({ open, selectedCap, setOpen }: EditCapModalProps) => {
  const { register, handleSubmit, control, reset } = useForm();
  const updateCapMutation = useUpdateCap();

  return (
    <Modal
      show={open}
      onClose={() => {
        setOpen(false);
        reset();
      }}
      dismissible
    >
      <ModalHeader className="border-gray-200">Edit Cap</ModalHeader>
      <ModalBody>
        <form
          onSubmit={handleSubmit((data) => {
            const boardIds = data.boardKeys.map((v: { value: string }) => v.value);
            const password = data.password ? data.password : undefined;

            updateCapMutation.mutate(
              {
                params: {
                  path: {
                    cap_id: selectedCap.id,
                  },
                },
                body: {
                  name: data.name,
                  description: data.description,
                  password,
                  board_ids: boardIds,
                },
              },
              {
                onSuccess: () => {
                  setOpen(false);
                  reset();
                },
              },
            );
          })}
        >
          <div className="flex flex-col">
            <Label>Name</Label>
            <TextInput
              placeholder="Name..."
              required
              defaultValue={selectedCap.name}
              {...register("name", { required: true })}
            />
          </div>
          <div className="flex flex-col mt-4">
            <Label>Description</Label>
            <TextInput
              placeholder="Description..."
              defaultValue={selectedCap.description}
              {...register("description")}
            />
          </div>
          <div className="flex flex-col mt-4">
            <Label>Password</Label>
            <TextInput placeholder="Password..." type="password" {...register("password")} />
          </div>
          <BoardMultiSelect
            control={control}
            name="boardKeys"
            defaultBoardIds={selectedCap.boardIds}
          />
          <div className="flex justify-end mt-4">
            <Button type="submit">Submit</Button>
          </div>
        </form>
      </ModalBody>
    </Modal>
  );
};

export default EditCapModal;
