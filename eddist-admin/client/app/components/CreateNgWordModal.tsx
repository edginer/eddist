import {
  Button,
  Label,
  Modal,
  ModalBody,
  ModalHeader,
  TextInput,
} from "flowbite-react";
import { useForm } from "react-hook-form";
import { toast } from "react-toastify";
import { createNgWord } from "~/hooks/queries";

interface CreateNgWordModalProps {
  open: boolean;
  setOpen: (open: boolean) => void;
  refetch: () => Promise<unknown>;
}

const CreateNgWordModal = ({
  setOpen,
  refetch,
  open,
}: CreateNgWordModalProps) => {
  const { register, handleSubmit } = useForm();

  return (
    <div>
      <Modal show={open} onClose={() => setOpen(false)}>
        <ModalHeader className="border-gray-200">Create NG Word</ModalHeader>
        <ModalBody>
          <form
            onSubmit={handleSubmit(async (data) => {
              try {
                const { mutate } = createNgWord({
                  body: {
                    name: data.name,
                    word: data.word,
                  },
                });
                await mutate();
                setOpen(false);
                toast.success("Successfully created NG word");
                await refetch();
              } catch (error) {
                toast.error("Failed to create NG word");
              }
            })}
          >
            <div className="flex flex-col">
              <Label>Name</Label>
              <TextInput
                placeholder="Name..."
                required
                {...register("name", {
                  required: true,
                })}
              />
            </div>
            <div className="flex flex-col mt-4">
              <Label>Word</Label>
              <TextInput
                placeholder="Word..."
                required
                {...register("word", {
                  required: true,
                })}
              />
            </div>
            <Button type="submit" className="mt-4">
              Create
            </Button>
          </form>
        </ModalBody>
      </Modal>
    </div>
  );
};

export default CreateNgWordModal;
